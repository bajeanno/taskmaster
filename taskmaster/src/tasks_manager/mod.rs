mod handle;
mod process;
mod routine;
mod tests;

use crate::process_handler::NominativeStatus;
use process::Process;
use routine::Client;
use tokio::sync::oneshot;

#[derive(Debug)]
pub enum ServerCommandError {
    NoSuchProgram(String),
}

pub enum TaskManagerCommand {
    ListProcesses(oneshot::Sender<Vec<NominativeStatus>>),
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
