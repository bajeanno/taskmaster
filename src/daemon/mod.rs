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

pub use claim_pid::start_mode::StartMode;
use daemonize::{Daemonized, DaemonizedOk, DefaultDaemonized, try_daemonized, try_daemonized_outside};

use libc::sys::types::Pid;
#[cfg(test)]
use test_utils::TestDir;

use crate::daemon::{config_state::ConfigState, tasks_manager::TaskManagerCommand};
use config::ProgramConfig;
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
pub unsafe fn run(start_mode: StartMode) -> Daemonized<(), Error, Pid, Error> {
    let config_state = try_daemonized_outside!(ConfigState::from_default_config_file());
    if !cfg!(debug_assertions) {
        match try_daemonized!(unsafe { daemonize() }) {
            DaemonizedOk::InsideDaemon(()) => {}
            DaemonizedOk::OutsideDaemon(pid) => {
                return Daemonized::OutsideDaemon(Ok(pid));
            }
        }
    }

    let pid_file_claim = match claim_pid::Claim::new(start_mode) {
        Ok(claim) => claim,
        Err(err) => return Daemonized::InsideDaemon(Err(err.into())),
    };

    let result = start_server(config_state);

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

fn start_server(config_state: ConfigState) -> Result<(), Error> {
    tokio::runtime::Runtime::new()
        .expect("Failed to init tokio runtime")
        .block_on(async {
            let handle = Arc::new(Mutex::new(tasks_manager::Routine::spawn(config_state)));

            let server_handle = crate::rpc::start_server(Arc::clone(&handle)).await?;

            signal_handling::handle_signal(handle).await?;

            server_handle.stop();

            Ok(())
        })
}
