mod command;
pub mod oneshot_command;
pub mod parsing;
// TODO remove this
mod placeholder;

use crate::rpc::ClientHandle;
use command::Command;

use crate::client::commands::placeholder::PlaceHolderError;

#[derive(thiserror::Error, Debug)]
#[allow(dead_code)] //TODO: Remove that
pub enum CommandExecutionError {
    #[error("No such program: `{0}`")]
    NoSuchProgram(String),
    #[error("PlaceHolder error: `{0}`")]
    PlaceHolderError(PlaceHolderError),
}

pub async fn send_command(
    cmd: Command,
    client: &ClientHandle,
) -> Result<(), CommandExecutionError> {
    cmd.send(client)
        .await
        .map_err(CommandExecutionError::PlaceHolderError)?;
    Ok(())
}
