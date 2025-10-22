mod commands;
mod session;
mod shell;

use crate::session::Session;
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let session = match Session::new().await {
        Ok(session) => session,
        Err(err) => {
            eprintln!("Failed to instanciate connection: {err}");
            return ExitCode::FAILURE;
        }
    };

    if std::env::args().nth(1).is_some() {
        match commands::oneshot_command::run(session).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(()) => ExitCode::FAILURE,
        }
    } else {
        match shell::run(session).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(()) => ExitCode::FAILURE,
        }
    }
}
