use crate::Program;
use thiserror::Error;
use tokio::process::Command;

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("{0}")]
    ParseError(#[from] shell_words::ParseError),
    #[error("Empty command for program {0}")]
    EmptyCommand(String),
}

pub(super) fn create_command(config: &Program) -> Result<Command, CommandError> {
    let parts = shell_words::split(&config.cmd)?;

    let mut parts_iter = parts.into_iter();
    let program = parts_iter
        .next()
        .ok_or_else(|| CommandError::EmptyCommand(String::from(config.name())))?;

    let mut command = Command::new(program);
    for arg in parts_iter {
        command.arg(arg);
    }

    if *config.clear_env() {
        command.env_clear();
    }
    config.env().iter().for_each(|(key, val)| {
        command.env(key, val);
    });

    Ok(command)
}
