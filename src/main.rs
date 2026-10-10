mod client;
mod daemon;
mod rpc;
mod utils;

use std::{process::ExitCode, thread::sleep, time::Duration};

use daemonize::Daemonized;
use logging::LogLevel;

use crate::{client::start_client, daemon::is_daemon_started, utils::print_error};

fn main() -> ExitCode {
    let mut maybe_rl = None;

    let is_daemon_started = match is_daemon_started() {
        Ok(ok) => ok,
        Err(err) => {
            eprintln!("Failed to check if daemon is started:");
            print_error(&err);
            return ExitCode::FAILURE;
        }
    };

    if !is_daemon_started {
        let mut rl = rustyline::DefaultEditor::new().unwrap();

        if !should_start_daemon(&mut rl) {
            return ExitCode::SUCCESS;
        }

        maybe_rl = Some(rl);

        match unsafe { daemon::run() } {
            Ok(Daemonized::IsInsideDaemon) => return ExitCode::SUCCESS,
            Ok(Daemonized::IsOutsideDaemon) => {
                // waiting a bit for the daemon to start before starting client
                sleep(Duration::from_millis(400));
            }
            Err(err) => {
                logging::log(&err, LogLevel::Error);
                return ExitCode::FAILURE;
            }
        }
    }

    start_client(maybe_rl)
}

fn should_start_daemon(rl: &mut rustyline::DefaultEditor) -> bool {
    loop {
        let line = rl
            .readline(
                "Daemon is not started, type yes to start it, no to cancel the command (Y/n): ",
            )
            .unwrap();

        match line.trim().to_lowercase().as_str() {
            "y" | "yes" | "" => return true,
            "n" | "no" => return false,
            _ => {
                eprintln!("Please write yes / no");
            }
        }
    }
}
