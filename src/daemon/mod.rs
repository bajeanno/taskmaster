mod error;
use std::sync::Arc;

pub use error::{ClaimError, Error};
mod config_state;
pub use config_state::ReloadType;
mod tasks_manager;
pub use tasks_manager::{Handle as TasksManagerHandle, ServerCommandError, TasksManagerStatus};

mod claim_pid;
mod config;
mod output_file;
mod process;
mod process_handler;
mod signal_handling;
#[cfg(test)]
mod test_utils;

use daemonize::{
    Daemonized, DaemonizedOk, DefaultDaemonized, try_daemonized, try_daemonized_outside,
};

use libc::sys::types::Pid;
#[cfg(test)]
use test_utils::TestDir;

use crate::daemon::{config_state::ConfigState, tasks_manager::TaskManagerCommand};
use config::ProgramConfig;
use std::{sync::Arc, time::Duration};
use tasks_manager::TaskManagerCommand;
use tokio::{
    sync::{Mutex, mpsc, oneshot},
    time::timeout,
};

pub type CommandReceiver = mpsc::UnboundedReceiver<(
    TaskManagerCommand,
    oneshot::Sender<Result<(), ServerCommandError>>,
)>;
pub type CommandSender = mpsc::UnboundedSender<(
    TaskManagerCommand,
    oneshot::Sender<Result<(), ServerCommandError>>,
)>;

pub fn is_daemon_started() -> Result<bool, ClaimError> {
    claim_pid::Claim::is_claimed()
}

/// # Safety
///
/// Only call when it is safe to call fork()
pub unsafe fn run() -> Daemonized<(), Error, Pid, Error> {
    let config_state = try_daemonized_outside!(ConfigState::from_default_config_file());

    if !cfg!(debug_assertions) {
        match try_daemonized!(unsafe { daemonize() }) {
            DaemonizedOk::InsideDaemon(()) => {}
            DaemonizedOk::OutsideDaemon(pid) => {
                return Daemonized::OutsideDaemon(Ok(pid));
            }
        }
    }

    let pid_file_claim = match claim_pid::Claim::new() {
        Ok(claim) => claim,
        Err(err) => return Daemonized::InsideDaemon(Err(err.into())),
    };

    let result = tokio::runtime::Runtime::new()
        .expect("Failed to init tokio runtime")
        .block_on(run_daemon(config_state));

    pid_file_claim.release_claim();

    Daemonized::InsideDaemon(result)
}

/// # Safety
///
/// Only call when it is safe to call fork()
unsafe fn daemonize() -> DefaultDaemonized {
    unsafe {
        daemonize::Daemonize::new()
            .stdout("/var/log/taskmaster.log")
            .stderr("/var/log/taskmaster.log")
            .start()
    }
}

async fn run_daemon(config_state: ConfigState) -> Result<(), Error> {
    // MAKE SURE WE DON'T RETURN FROM THIS FUNCTION TILL WE SENT THE SHUTDOWN COMMAND AND AWAITED
    // THE ROUTINE STOPPED RECEIVER
    let (task_manager_handle, mut routine_stopped_receiver) =
        tasks_manager::Routine::spawn(config_state);
    let task_manager_handle = Arc::new(Mutex::new(task_manager_handle));

    tokio::select! {
        _ = routine_stopped_receiver.recv() => Ok(()),

        err = run_daemon_services(Arc::clone(&task_manager_handle)) => {
            let _ = task_manager_handle
                .lock()
                .await
                .shutdown()
                .await;

            if timeout(Duration::from_secs(5), routine_stopped_receiver.recv()).await.is_err() {
                logging::error!("Routine shutdown timed-out");
            }

            Err(err)
        }
    }
}

async fn run_daemon_services(task_manager_handle: Arc<Mutex<tasks_manager::Handle>>) -> Error {
    let _server_handle = match crate::rpc::start_server(Arc::clone(&task_manager_handle)).await {
        Ok(server_handle) => server_handle,
        Err(err) => return err.into(),
    };

    let (handle_signal_error_sender, handle_signals_error_receiver) = oneshot::channel();

    let handle_signals_join_handle = tokio::spawn(async move {
        signal_handling::handle_signals(task_manager_handle, handle_signal_error_sender).await
    });

    match handle_signals_error_receiver.await {
        Ok(err) => Error::Signal(err),
        Err(_) => {
            timeout(Duration::from_secs(5), handle_signals_join_handle)
                .await
                .expect("joining handle_signals timed-out")
                // The expect below should always panic
                .expect("handle_signals task failed");
            unreachable!(
                "handle signals task should never stop, nor should it ever drop it's error sender"
            )
        }
    }
}
