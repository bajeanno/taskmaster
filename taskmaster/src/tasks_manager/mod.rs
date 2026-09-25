mod handle;
mod process_list;
mod process_registry;
mod routine;

#[cfg(test)]
mod tests;

use crate::{
    config_state::{InitFileError, ReloadArgs},
    tasks_manager::process_list::ProcessList,
};
use routine::Client;
use thiserror::Error;
use tokio::sync::oneshot;

#[derive(Debug, Error)]
pub enum ServerCommandError {
    #[error("Task not found in current config: {0}")]
    NoSuchProgram(String),

    #[error("error loading config from file '{config_file_path}': {error}")]
    FailedToLoadNewConfig {
        error: String,
        config_file_path: String,
    },

    #[error("{0}")] // Error message is already contained in sub-type
    InitFileError(#[from] InitFileError),
}

pub enum TaskManagerCommand {
    ListProcesses(oneshot::Sender<ProcessList>),
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
