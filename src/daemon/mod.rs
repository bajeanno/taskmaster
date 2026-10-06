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
mod tests;

use daemonize::Daemonized;

#[cfg(test)]
use tests::TestDir;

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
pub unsafe fn run() -> Result<Daemonized, Error> {
    let pid_file_claim = claim_pid::Claim::new()?;

    if !cfg!(debug_assertions) {
        match unsafe { daemonize()? } {
            Daemonized::IsInsideDaemon => {}
            Daemonized::IsOutsideDaemon => return Ok(Daemonized::IsOutsideDaemon),
        }
    }

    let result = start_server();

    pid_file_claim.release_claim();

    result.map(|()| Daemonized::IsInsideDaemon)
}

/// # Safety
///
/// Only call when it is safe to call fork()
unsafe fn daemonize() -> Result<Daemonized, Error> {
    unsafe {
        daemonize::Daemonize::new()
            .stdout("/var/log/taskmaster.log")
            .stderr("/var/log/taskmaster.log")
            .start()
            .map_err(Into::into)
    }
}

fn start_server() -> Result<(), Error> {
    let _config_manager = ConfigState::from_default_config_file();

    tokio::runtime::Runtime::new()
        .expect("Failed to init tokio runtime")
        .block_on(async {
            // TODO: spawn task manager and create signal handling task
            crate::rpc::start_server()
                .await?
                .wait_until_stopped()
                .await
                .unwrap();

            Ok(())
        })
}
