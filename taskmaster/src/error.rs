use std::fmt::Display;

use crate::server;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    PortArgumentIsNotAnInteger {
        input: String,
        error: std::num::ParseIntError,
    },

    #[allow(dead_code)]
    FailedToDaemonize(daemonize::Error),

    #[allow(dead_code)]
    TaskServerFailure(server::Error),
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

impl core::error::Error for Error {}

impl From<daemonize::Error> for Error {
    fn from(error: daemonize::Error) -> Self {
        Self::FailedToDaemonize(error)
    }
}

impl From<server::Error> for Error {
    fn from(error: server::Error) -> Self {
        Self::TaskServerFailure(error)
    }
}
