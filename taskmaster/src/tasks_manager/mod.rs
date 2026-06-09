mod handle;
mod process;
mod routine;

#[cfg(test)]
mod tests;

use crate::process_handler::NominativeStatus;
use process::Process;
use routine::Client;
use tokio::sync::oneshot;

#[derive(Debug)]
pub enum ServerCommandError {
    NoSuchProgram(String),
    LoadError(String),
}

pub enum TaskManagerCommand {
    ListProcesses(oneshot::Sender<Vec<Vec<NominativeStatus>>>),
    Reload(String),
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
