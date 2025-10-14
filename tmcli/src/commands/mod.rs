mod command;
pub mod oneshot_command;
pub mod parsing;
// TODO remove this
mod placeholder;

use command::Command;
#[allow(unused_imports)]
use thiserror::Error;

use crate::{commands::placeholder::PlaceHolderError, session::Session};

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum CommandExecutionError {
    #[error("No such program: `{0}`")]
    NoSuchProgram(String),
    #[error("`{0}`")]
    RequestError(#[from] connection::Error),
    #[error("PlaceHolder error: `{0}`")]
    PlaceHolderError(PlaceHolderError),
}

pub async fn send_command(cmd: Command, session: &Session) -> Result<(), CommandExecutionError> {
    cmd.send(session)
        .await
        .map_err(CommandExecutionError::PlaceHolderError)?;
    Ok(())
}
