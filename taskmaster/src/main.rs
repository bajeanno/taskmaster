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

#[cfg(test)]
use tests::TestDir;

use crate::{config_state::ConfigState, tasks_manager::ServerCommandError};
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

fn main() {
    let _ = entrypoint().inspect_err(|err| eprintln!("{err}"));
}

fn entrypoint() -> Result<(), Error> {
    let pid_file_claim = claim_pid::Claim::new()?;

    if !cfg!(debug_assertions) {
        daemonize()?
    }

    let result = start_server();
    pid_file_claim.release_claim();
    result
}

fn daemonize() -> Result<(), Error> {
    unsafe {
        daemonize::Daemonize::new()
            .stdout("./server_output")
            .stderr("./server_output")
            .start()?
    }
    Ok(())
}

fn start_server() -> Result<(), Error> {
    let config_state = ConfigState::from_default_config_file()?;

    tokio::runtime::Runtime::new()
        .expect("Failed to init tokio runtime")
        .block_on(async {
            let handle = tasks_manager::Routine::spawn(config_state);

            let server_handle = shared_code::rpc::start_server().await?;

            signal_handling::handle_signal(&handle).await?;

            server_handle.stop();

            Ok(())
        })
}
