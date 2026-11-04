mod handle;
pub use handle::Handle;
mod status;
pub use status::TasksManagerStatus;
mod process_registry;
mod routine;
pub use routine::{Routine, RoutineStoppedReceiver};

#[cfg(test)]
mod tests;

use crate::daemon::config_state::{InitFileError, ReloadType, RestoreConfigFileError};
use routine::Client;
use tokio::sync::oneshot;

#[derive(Debug, thiserror::Error, serde::Serialize, serde::Deserialize)]
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

    #[error("Error restoring config file")]
    RestoringConfigFile(#[from] RestoreConfigFileError),

    #[error("Shutdown in progress")]
    ShutdownInProgress,
}

// TODO remove allow dead code
#[allow(dead_code)]
pub enum TaskManagerCommand {
    ListProcesses(oneshot::Sender<TasksManagerStatus>),
    Reload(ReloadType),
    StartProgram { task_name: String },
    RestartProgram { task_name: String },
    StopProgram { task_name: String },
    SubscribeToProgramEvents { task_name: String, client: Client },
    UnsubscribeToProgramEvents { task_name: String, client: Client },
    StopAllProcesses,
    RestoreConfigFile,
    Shutdown,
}
