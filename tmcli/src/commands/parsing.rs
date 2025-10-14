use thiserror::Error;

use super::command::Command;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error(
        "Bad command name: `{command}`\naccepted command names are :\n\
            \tstatus\n\
            \tstop\n\
            \tstart\n\
            \trestart\n\
            \tshutdown\n\
            \treload"
    )]
    BadCommand { command: String },
    #[error("Missing argument")]
    MissingArgument,
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
        command => Err(ParseError::BadCommand {
            command: command.to_string(),
        }),
    }
}
