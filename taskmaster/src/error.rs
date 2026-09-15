use std::{fmt::Display, num::ParseIntError};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    PortArgumentIsNotAnInteger {
        input: String,
        error: std::num::ParseIntError,
    },

    #[allow(dead_code)] //TODO: remove that
    FailedToDaemonize(daemonize::Error),

    #[allow(dead_code)] //TODO: remove that
    TaskServerFailure,

    Pid(#[from] PidError),
}

#[derive(Debug, Error)]
pub enum PidError {
    #[error("Failed to open pid file: {0}")]
    OpenFile(std::io::Error),
    #[error("Failed to read pid file: {0}")]
    ReadFile(std::io::Error),
    #[error("Failed to write to pid file: {0}")]
    WriteFile(std::io::Error),
    #[error("Failed to parse pid file content: {0}")]
    Parse(ParseIntError),
    #[error("Another instance of Taskmaster is already running")]
    OtherInstanceRunning,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PortArgumentIsNotAnInteger { input, error } => {
                write!(
                    f,
                    "Failed to parse port number from input: '{input}': {error}"
                )
            }
            _ => write!(f, "{self:#?}"),
        }
    }
}

impl From<daemonize::Error> for Error {
    fn from(error: daemonize::Error) -> Self {
        Self::FailedToDaemonize(error)
    }
}
