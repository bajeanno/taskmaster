mod command;
pub mod parsing;
pub mod session;

use command::Command;
use thiserror::Error;
#[allow(unused_imports)]
use session::{ConnectError, Session};

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum ServerError {
    #[error("No such program: `{0}`")]
    NotFound(String),
    #[error("`{0}`")]
    ConnectError(ConnectError),
    #[error("`{0}`")]
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

pub async fn send_command(cmd: Command, session: &Session) -> Result<(), ServerError> {
    let _ = cmd
        .send(session)
        .await
        .inspect_err(|err| eprintln!("Error sending command: {err}"));
    Ok(())
}
