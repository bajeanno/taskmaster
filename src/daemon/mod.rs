mod error;
pub use error::{ClaimError, Error};
mod claim_pid;
mod config;
mod config_state;
pub use config_state::ReloadType;
mod output_file;
mod process;
mod process_handler;
mod signal_handling;
mod tasks_manager;
pub use tasks_manager::{
    Handle as TasksManagerHandle, ServerCommandError, TaskManagerCommand, TasksManagerStatus,
};

#[cfg(test)]
mod test_utils;

use daemonize::{Daemonized, DaemonizedOk, DefaultDaemonized, try_daemonized};

use libc::sys::types::Pid;
#[cfg(test)]
use test_utils::TestDir;

use crate::daemon::config_state::ConfigState;
use config::ProgramConfig;
use std::{panic, sync::Arc};
use tokio::sync::{Mutex, mpsc, oneshot};

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

    let result = start_server();

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

fn start_server() -> Result<(), Error> {
    let config_state = ConfigState::from_default_config_file()?;

    tokio::runtime::Runtime::new()
        .expect("Failed to init tokio runtime")
        .block_on(async {
            let (handle_signal_error_sender, handle_signal_error_receiver) = oneshot::channel();

            let (handle, mut routine_stopped_receiver) =
                tasks_manager::Routine::spawn(config_state);
            // TODO use send should always be sent on drop
            let handle = Arc::new(Mutex::new(handle));

            let _server_handle = crate::rpc::start_server(Arc::clone(&handle)).await?;

            let handle_clone = Arc::clone(&handle);
            let handle_signals_join_handle = tokio::spawn(async move {
                signal_handling::handle_signals(&handle_clone, handle_signal_error_sender).await
            });

            tokio::select! {
                _ = routine_stopped_receiver.recv() => { }

                msg_result = handle_signal_error_receiver => {
                    let err = msg_result.expect("handle_signals should never drop the sender");
                    let _ = handle.lock().await.send(TaskManagerCommand::Shutdown).await;
                    let _ = routine_stopped_receiver.recv().await;
                    return Err(Error::Signal(err));
                }
            };

            handle_signals_join_handle.abort();
            match handle_signals_join_handle.await {
                Ok(_) => todo!(),
                Err(err) => {
                    if let Ok(reason) = err.try_into_panic() {
                        panic::resume_unwind(reason);
                    }
                }
            }

            Ok(())
        })
}
