use crate::{
    Session,
    commands::{
        CommandExecutionError,
        parsing::{ParseError, parse_command},
        send_command,
    },
};

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Failed to parse command: {0}")]
    Parsing(#[from] ParseError),
    #[error("Failed to execute command: {0}")]
    Execution(#[from] CommandExecutionError),
    #[error("Command is empty")]
    EmptyCommand,
}

pub async fn run(session: Session) -> Result<(), ()> {
    let Some(command) =
        parse_command(std::env::args().skip(1)).map_err(|err| eprintln!("{err}"))?
    else {
        eprintln!("{}", Error::EmptyCommand);
        return Err(());
    };
    send_command(command, &session)
        .await
        .map_err(|err| eprintln!("{err}"))
}
