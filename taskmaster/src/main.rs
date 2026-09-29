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

use crate::{
    config_state::ConfigState, tasks_manager::ServerCommandError,
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
    let _config_state = match ConfigState::from_default_config_file() {
        Ok(config_state) => config_state,
        Err(err) => todo!("{err}"), // TODO: handle InitFileError
    };

    tokio::runtime::Runtime::new()
        .expect("Failed to init tokio runtime")
        .block_on(async {
            // TODO: spawn task manager and create signal handling task
            shared_code::rpc::start_server()
                .await?
                .wait_until_stopped()
                .await
                .unwrap();

            Ok(())
        })
}
