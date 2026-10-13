mod claim_pid;
mod config;
mod config_state;
mod error;
mod output_file;
mod process;
mod process_handler;
mod signal_handling;
mod tasks_manager;

#[cfg(test)]
mod test_utils;

use daemonize::{Daemonized, DaemonizedOk, DefaultDaemonized, try_daemonized};

use libc::sys::types::Pid;
#[cfg(test)]
use test_utils::TestDir;

use crate::daemon::{
    config_state::ConfigState, error::ClaimError, tasks_manager::ServerCommandError,
};
use config::ProgramConfig;
use error::Error;
use tasks_manager::TaskManagerCommand;
use tokio::sync::{mpsc, oneshot};

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
            let handle = tasks_manager::Routine::spawn(config_state);

            let server_handle = crate::rpc::start_server().await?;

            signal_handling::handle_signal(&handle).await?;

            server_handle.stop();

            Ok(())
        })
}
