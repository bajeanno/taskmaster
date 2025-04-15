use std::fmt::Display;
use std::error::Error;

use commands::ServerCommand;

#[derive(Debug)]
pub enum ParseError {
    BadCommand(String),
    MissingArgument,
}

impl Error for ParseError {}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadCommand(str) => write!(f, "Bad command name: {str}\naccepted command names are :\n\tstop\n\tstart\n\trestart\n\tshutdown\n\treload"),
            Self::MissingArgument => write!(f, "Missing argument"),
        }
    }
}

pub fn parse_command(mut args: impl Iterator<Item = String>) -> Result<ServerCommand, ParseError> {
    match args.next().ok_or_else(|| ParseError::MissingArgument)?.as_str().trim() {
        "start" => {
            let program = args.next().ok_or_else(|| ParseError::MissingArgument)?;
            Ok(ServerCommand::StartProgram(program))
        },
        "stop" => {
            let program = args.next().ok_or_else(|| ParseError::MissingArgument)?;
            Ok(ServerCommand::StopProgram(program))
        },
        "restart" => {
            let program = args.next().ok_or_else(|| ParseError::MissingArgument)?;
            Ok(ServerCommand::RestartProgram(program))
        },
        "shutdown" => Ok(ServerCommand::StopDaemon),
        "reload" => Ok(ServerCommand::ReloadConfigFile),
        command => Err(ParseError::BadCommand(command.to_string())),
    }
}
