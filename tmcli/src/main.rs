mod client;
mod shell;

use std::{env::args, fmt::Display};

use client::{
    ServerError,
    parsing::{ParseError, parse_command},
    send_command,
};

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

fn entrypoint() -> Result<(), ClientError> {
    let command = parse_command(args().skip(1)).map_err(|err| ClientError::ParseError(err))?;
    println!("{command:?}");
    send_command(command).map_err(|err| ClientError::ServerError(err));
    Ok(())
}
