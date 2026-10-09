mod handle;
mod process_list;
mod process_registry;
mod routine;

#[cfg(test)]
mod tests;

use crate::daemon::{
    config_state::{InitFileError, ReloadArgs},
    tasks_manager::process_list::ProcessList,
};
pub use handle::Handle;
use routine::Client;
pub use routine::Routine;
use tokio::sync::oneshot;

#[derive(Debug, thiserror::Error)]
pub enum ServerCommandError {
    #[error("Task not found in current config: {0}")]
    NoSuchProgram(String),

    #[error("error loading config from file '{config_file_path}': {error}")]
    FailedToLoadNewConfig {
        error: String,
        config_file_path: String,
    },

    #[error("Failed to load configuration from init file")]
    InitFileError(#[from] InitFileError),
}

// TODO remove allow dead code
#[allow(dead_code)]
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
