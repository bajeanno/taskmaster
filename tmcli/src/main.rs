mod client;
mod shell;

use std::env::args;

use client::{
    ServerError,
    parsing::{ParseError, parse_command},
};

use crate::client::{send_command, session::Session};
use thiserror::Error;

#[derive(Error, Debug)]
enum ClientError {
    #[error("`{0}`")]
    ServerError(ServerError),
    #[error("`{0}`")]
    ParseError(ParseError),
}

impl From<ParseError> for ClientError {
    fn from(error: ParseError) -> Self {
        Self::ParseError(error)
    }
}

impl From<ServerError> for ClientError {
    fn from(error: ServerError) -> Self {
        Self::ServerError(error)
    }
}

async fn entrypoint() -> Result<(), ClientError> {
    let Some(command) = parse_command(args().skip(1)).map_err(ClientError::ParseError)? else {
        eprintln!("Command is empty");
        return Err(ClientError::ParseError(ParseError::MissingArgument));
    };
    let session = Session::new().await.map_err(ServerError::ConnectError)?;
    send_command(command, &session)
        .await
        .map_err(ClientError::ServerError)?;
    Ok(())
}

#[tokio::main]
async fn main() {
    if std::env::args().nth(1).is_some() {
        entrypoint().await.unwrap_or_else(|err| eprintln!("{err}"));
    } else {
        shell::run().await.unwrap_or_else(|err| eprintln!("{err}"));
    }
}
