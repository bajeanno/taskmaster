pub mod parsing;
mod session;

use std::fmt::Display;

use commands::{ClientCommand, ServerCommand};
use session::{ConnectError, Session};

#[derive(Debug)]
pub enum ServerError {
    NotFound(String),
    ConnectError(ConnectError),
}

impl From<ConnectError> for ServerError {
    fn from(value: ConnectError) -> Self {
        Self::ConnectError(value)
    }
}

impl Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(program) => write!(f, "No such program: {program}"),
            Self::ConnectError(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for ServerError {}
