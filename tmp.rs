mod commands;
mod session;
mod shell;

use std::process::ExitCode;
use crate::session::Session;

#[tokio::main]
async fn main() -> ExitCode {
    let session = match Session::new().await {
        Ok(session) => session,
        Err(err) => ExitCode::FAILURE,
    };

    if std::env::args().nth(1).is_some() {
        match commands::oneshot_command::run(session).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(()) => ExitCode::FAILURE,
        }
    } else {
        match shell::run(session).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(()) => ExitCode::FAILURE
        }
    }
}
