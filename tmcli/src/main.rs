mod client;
mod shell;

use std::{env::args, fmt::Display};

use client::{
    ServerError,
    parsing::{ParseError, parse_command},
};

use crate::client::send_command;

enum ClientError {
    ServerError(ServerError),
    ParseError(ParseError),
}

impl Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ServerError(err) => write!(f, "{err}"),
            Self::ParseError(err) => write!(f, "{err}"),
        }
    }
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
    let Some(command) =
        parse_command(args().skip(1)).map_err(ClientError::ParseError)?
    else {
        eprintln!("Command is empty");
        return Err(ClientError::ParseError(ParseError::MissingArgument));
    };
    send_command(command)
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
