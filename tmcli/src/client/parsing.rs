use std::error::Error;
use std::fmt::Display;

use super::command::Command;

#[derive(Debug)]
pub enum ParseError {
    BadCommand(String),
    MissingArgument,
}

impl Error for ParseError {}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadCommand(str) => write!(
                f,
                "Bad command name: {str}\naccepted command names are :\n\tstatus\n\tstop\n\tstart\n\trestart\n\tshutdown\n\treload"
            ),
            Self::MissingArgument => write!(f, "Missing argument"),
        }
    }
}

pub fn parse_command(
    mut args: impl Iterator<Item = String>,
) -> Result<Option<Command>, ParseError> {
    match args
        .next()
        .ok_or(ParseError::MissingArgument)?
        .as_str()
        .trim()
    {
        "status" => Ok(Some(Command::ListTasks)),
        "start" => {
            let program = args.next().ok_or(ParseError::MissingArgument)?;
            //TODO: args handling
            Ok(Some(Command::StartProgram(program)))
        }
        "stop" => {
            let program = args.next().ok_or(ParseError::MissingArgument)?;
            Ok(Some(Command::StopProgram(program)))
        }
        "restart" => {
            let program = args.next().ok_or(ParseError::MissingArgument)?;
            Ok(Some(Command::RestartProgram(program)))
        }
        "shutdown" => Ok(Some(Command::StopDaemon)),
        "reload" => Ok(Some(Command::ReloadConfigFile)),
        "" => Ok(None),
        command => Err(ParseError::BadCommand(command.to_string()))
    }
}
