mod commands;
mod shell;

use std::{process::ExitCode, time::Duration};

use logging::LogLevel;
use tokio::time::sleep;

use crate::rpc;

pub fn start_client(maybe_rl: Option<rustyline::DefaultEditor>) -> ExitCode {
    tokio::runtime::Runtime::new()
        .expect("Failed to init tokio runtime")
        .block_on(async {
            let Ok(client) = tokio::time::timeout(Duration::from_secs(2), async {
                loop {
                    match rpc::connect_client().await {
                        Ok(client) => return client,
                        Err(err) => {
                            logging::log_with_msg(
                                "Failed to connect to taskmaster daemon",
                                &err,
                                LogLevel::Warning,
                            );
                            sleep(Duration::from_millis(400)).await;
                        }
                    }
                }
            })
            .await
            else {
                logging::error!("Connections attempts timeouted");
                return ExitCode::FAILURE;
            };

            if std::env::args().nth(1).is_some() {
                match commands::oneshot_command::run(client).await {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(()) => ExitCode::FAILURE,
                }
            } else {
                match shell::run(client, maybe_rl).await {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(()) => ExitCode::FAILURE,
                }
            }
        })
}
