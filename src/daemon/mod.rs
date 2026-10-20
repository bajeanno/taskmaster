mod error;

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

use crate::daemon::{
    config_state::ConfigState,
    signal_handling::SignalError,
    tasks_manager::{RoutineStoppedReceiver, TaskManagerCommand},
};
use config::ProgramConfig;
use std::{panic, pin::Pin, sync::Arc, time::Duration};
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

struct FutureToJoin {
    future: Pin<Box<dyn Future<Output = ()>>>,
    name: &'static str,
}

async fn run_daemon(config_state: ConfigState) -> Result<(), Error> {
    let mut futures: Vec<FutureToJoin> = Vec::new();
    futures.reserve_exact(3);

    let (tasks_manager, routine_stopped_receiver) =
        start_tasks_manager(&mut futures, config_state).await;

    futures = start_server(futures, Arc::clone(&tasks_manager)).await?;

    let handle_signals_error_receiver =
        start_handle_signals_loop(&mut futures, tasks_manager).await;

    let result = tokio::select! {
        _ = routine_stopped_receiver => {
            Ok(())
        }

        msg = handle_signals_error_receiver => {
            match msg {
                Ok(err) => Err(Error::Signal(err)),
                Err(_) => {
                    // error sender was dropped, meaning the loop ended, meaning sigint was
                    // received and shutdown is in progress
                    Ok(())
                }
            }
        }
    };

    join_all(futures).await;
    result
}

async fn start_tasks_manager(
    futures: &mut Vec<FutureToJoin>,
    config_state: ConfigState,
) -> (Arc<Mutex<tasks_manager::Handle>>, RoutineStoppedReceiver) {
    let (tasks_manager, tasks_manager_routine_join_handle, routine_stopped_receiver) =
        tasks_manager::Routine::spawn(config_state);
    let tasks_manager = Arc::new(Mutex::new(tasks_manager));

    let tasks_manager_handle_clone = Arc::clone(&tasks_manager);
    futures.push(FutureToJoin {
        future: Box::pin(async move {
            tasks_manager_handle_clone
                .lock()
                .await
                .shutdown()
                .await
                .expect("Failed to send shutdown msg to tasks manager");
            tasks_manager_routine_join_handle
                .await
                .expect("Failed to join tasks manager");
        }),
        name: "tasks manager",
    });

    (tasks_manager, routine_stopped_receiver)
}

async fn start_server(
    mut futures: Vec<FutureToJoin>,
    tasks_manager: Arc<Mutex<tasks_manager::Handle>>,
) -> Result<Vec<FutureToJoin>, Error> {
    let server_handle = match crate::rpc::start_server(tasks_manager).await {
        Ok(server_handle) => server_handle,
        Err(err) => {
            join_all(futures).await;
            return Err(err.into());
        }
    };

    futures.push(FutureToJoin {
        future: Box::pin(async move {
            server_handle.stop();
        }),
        name: "server handle",
    });

    Ok(futures)
}

async fn start_handle_signals_loop(
    futures: &mut Vec<FutureToJoin>,
    tasks_manager: Arc<Mutex<tasks_manager::Handle>>,
) -> oneshot::Receiver<SignalError> {
    let (handle_signal_error_sender, handle_signals_error_receiver) = oneshot::channel();

    let handle_signals_join_handle = tokio::spawn(signal_handling::handle_signals(
        tasks_manager,
        handle_signal_error_sender,
    ));

    futures.push(FutureToJoin {
        future: Box::pin(async move {
            handle_signals_join_handle.abort();

            if let Err(err) = handle_signals_join_handle.await
                && let Ok(err) = err.try_into_panic()
            {
                panic::resume_unwind(err);
            }
        }),
        name: "handle signals",
    });

    handle_signals_error_receiver
}

async fn join_all(futures: Vec<FutureToJoin>) {
    for FutureToJoin { future, name } in futures.into_iter().rev() {
        if timeout(Duration::from_secs(5), future).await.is_err() {
            logging::error!("Failed to join {name}'s handle")
        }
    }
}
