mod handle;
mod process_registry;
mod routine;

#[cfg(test)]
mod tests;

use crate::{
    config_state::{InitFileError, ReloadArgs},
    process_handler::NominativeStatus,
};
use routine::Client;
use thiserror::Error;
use tokio::sync::oneshot;

#[derive(Debug, Error)]
pub enum ServerCommandError {
    #[error("{0}")]
    NoSuchProgram(String),
    #[error("error loading config from file '{config_file_path}': {error}")]
    FailedToLoadNewConfig {
        error: String,
        config_file_path: String,
    },
    #[error("{0}")]
    InitFileError(#[from] InitFileError),
}

pub enum TaskManagerCommand {
    ListProcesses(oneshot::Sender<Vec<Vec<NominativeStatus>>>),
    Reload(ReloadArgs),
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
