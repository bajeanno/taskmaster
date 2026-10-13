use crate::rpc::ClientHandle;

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Command can not be empty")]
    EmptyCommand,
    #[error("Unknown command: {0}")]
    UnknownCommand(String),
    #[error("UnexpectedArg: {0}")]
    UnexpectedArg(String),
    #[error("Missing argument")]
    MissingArgument,
}

pub async fn handle_command(
    _client: &ClientHandle,
    mut args: impl Iterator<Item = String>,
) -> Result<(), ParseError> {
    let command_name = args.next().ok_or(ParseError::EmptyCommand)?;
    match command_name.as_str() {
        "status" => {
            check_no_additional_args(args)?;
            todo!();
        }
        "reload" => {
            todo!();
        }
        "start" => {
            let _program = args.next().ok_or(ParseError::MissingArgument)?;
            check_no_additional_args(args)?;
            todo!();
        }
        "restart" => {
            let _program = args.next().ok_or(ParseError::MissingArgument)?;
            check_no_additional_args(args)?;
            todo!();
        }
        "stop" => {
            let _program = args.next().ok_or(ParseError::MissingArgument)?;
            check_no_additional_args(args)?;
            todo!();
        }
        "shutdown" => {
            let _program = args.next().ok_or(ParseError::MissingArgument)?;
            check_no_additional_args(args)?;
            todo!();
        }
        "" => Err(ParseError::EmptyCommand),
        _ => Err(ParseError::UnknownCommand(command_name)),
    }
}

fn check_no_additional_args(mut args: impl Iterator<Item = String>) -> Result<(), ParseError> {
    match args.next() {
        Some(arg) => Err(ParseError::UnexpectedArg(arg)),
        None => Ok(()),
    }
}
