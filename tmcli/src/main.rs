mod commands;
mod shell;

use std::process::ExitCode;

use shared_code::{rpc, utils};

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Failed to connect to taskmaster daemon")]
    ConnectingToDaemon(#[from] rpc::ConnectClientError),
}

#[tokio::main]
async fn main() -> ExitCode {
    let client = match rpc::connect_client().await {
        Ok(client) => client,
        Err(err) => {
            utils::print_error(&Error::ConnectingToDaemon(err));
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
