mod client;
mod shell;

use std::{env::args, process::ExitCode};
use client::{parsing::parse_command, send_command};
use shell::shell::ShellError;
use commands::ServerCommand;

fn wrap_single_cmd(args: Vec<String>) -> Result<(), ShellError> {
    let command: ServerCommand = parse_command(args)?;
    send_command(command);
    return Ok(());
}

fn main() -> ExitCode {
    if let Some(_) = args().nth(1) {
        let args = {
            args().fold(Vec::new(), |mut acc: Vec<String>, value: String| {
                acc.push(value.clone());
                acc
            })
        };
        let ret = wrap_single_cmd(args);
        if let Err(err) = ret {
            return ExitCode::from(err.get_code());
        }
        return ExitCode::from(0);
    } else {
        let ret = shell::run();
        if let Err(err) = ret {
            return ExitCode::from(err.get_code());
        }
        return ExitCode::from(0);
    }
}
