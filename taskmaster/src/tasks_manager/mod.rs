mod handle;
mod process;
mod routine;

#[cfg(test)]
mod tests;

use crate::process_handler::NominativeStatus;
use process::Process;
use routine::Client;
use tokio::sync::{mpsc, oneshot};

#[allow(dead_code)]
#[derive(Debug)]
// TODO: use thiserror
pub enum ServerCommandError {
    NoSuchProgram(String),
    FailedToLoadNewConfig(String),
}

#[allow(dead_code)]
pub enum TaskManagerCommand {
    ListProcesses(oneshot::Sender<Vec<Vec<NominativeStatus>>>),
    Reload {
        config_file_name: String,
    },
    StartProgram {
        program_name: String,
    },
    RestartProgram {
        program_name: String,
    },
    StopProgram {
        program_name: String,
    },
    SubscribeToProgramEvents {
        program_name: String,
        client: Client,
    },
    UnsubscribeToProgramEvents {
        program_name: String,
        client: Client,
    },
    StopAllProcesses,
    Exit,
}

pub type CommandReceiver = mpsc::UnboundedReceiver<(
    TaskManagerCommand,
    oneshot::Sender<core::result::Result<(), ServerCommandError>>,
)>;
pub type CommandSender = mpsc::UnboundedSender<(
    TaskManagerCommand,
    oneshot::Sender<core::result::Result<(), ServerCommandError>>,
)>;
