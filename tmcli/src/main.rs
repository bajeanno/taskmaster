mod commands;
mod shell;

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let client = match shared_code::rpc::connect_client().await {
        Ok(client) => client,
        Err(err) => {
            eprintln!("Failed to connect to taskmaster daemon: {err}");
            return ExitCode::FAILURE;
        }
    };

    if std::env::args().nth(1).is_some() {
        match commands::oneshot_command::run(client).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(()) => ExitCode::FAILURE,
        }
    } else {
        match shell::run(client).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(()) => ExitCode::FAILURE,
        }
    }
}
