mod command;
pub mod parsing;
mod session;

use command::Command;
#[allow(unused_imports)]
use session::{ConnectError, Session};
use std::fmt::Display;

#[derive(Debug)]
#[allow(dead_code)] //todo remove this
pub enum ServerError {
    NotFound(String),
    ConnectError(ConnectError),
    RequestError(connection::Error),
}

impl From<ConnectError> for ServerError {
    fn from(value: ConnectError) -> Self {
        Self::ConnectError(value)
    }
}

impl From<connection::Error> for ServerError {
    fn from(value: connection::Error) -> Self {
        Self::RequestError(value)
    }
}

impl Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(program) => write!(f, "No such program: {program}"),
            Self::ConnectError(err) => write!(f, "{err}"),
            Self::RequestError(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for ServerError {}

pub async fn send_command(cmd: Command) -> Result<(), ServerError> {
    let session = Session::new().await?;
    let _ = cmd
        .send(session)
        .await
        .inspect_err(|err| eprintln!("Error sending command: {err}"));
    Ok(())
}
