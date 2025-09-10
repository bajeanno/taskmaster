pub mod parsing;
mod session;

use std::fmt::Display;

#[allow(unused_imports)]
use commands::{ClientCommand, ServerCommand};
#[allow(unused_imports)]
use session::{ConnectError, Session};

#[derive(Debug)]
pub enum ServerError {
    NotFound(String),
    ConnectError(ConnectError),
    RequestError,
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
            Self::RequestError => write!(f, "Request Error"),
        }
    }
}

impl std::error::Error for ServerError {}

pub async fn send_command(cmd: ServerCommand) -> Result<(), ServerError> {
    let mut session = Session::new().await?;
    session.request(cmd).await?;
    Ok(())
}