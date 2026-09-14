use std::fmt::Display;

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
    #[error("{0}")]
    File(#[from] std::io::Error),
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
