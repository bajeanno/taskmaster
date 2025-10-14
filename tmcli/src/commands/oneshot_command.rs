use crate::{
    Session,
    commands::{
        CommandExecutionError,
        parsing::{ParseError, parse_command},
        send_command,
    },
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to parse command: {0}")]
    Parsing(#[from] ParseError),
    #[error("Failed to execute command: {0}")]
    Execution(#[from] CommandExecutionError),
}

pub async fn run(session: Session) -> Result<(), Error> {
    let Some(command) = parse_command(std::env::args().skip(1))? else {
        eprintln!("Command is empty");
        return Err(Error::Parsing(ParseError::MissingArgument));
    };
    send_command(command, &session).await?;
    Ok(())
}
