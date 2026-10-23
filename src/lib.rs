mod client;
mod daemon;
mod rpc;

use std::{process::ExitCode, thread::sleep, time::Duration};

use daemon::StartMode;
use daemonize::{Daemonized, DaemonizedOk, try_daemonized, try_daemonized_outside};
use logging::LogLevel;
use rustyline::error::ReadlineError;

use crate::daemon::{ClaimError, is_daemon_started};

#[derive(Debug, thiserror::Error)]
enum Error {
    // Daemon errors
    #[error("Failed to check if daemon is started")]
    CheckingIfDaemonIsStarted(#[from] ClaimError),
    #[error("Failed to start daemon")]
    StartingDaemon(#[from] daemon::Error),

    // Client errors
    #[error("Failed to connect to taskmaster daemon")]
    ConnectingToDaemon(#[from] rpc::ConnectClientError),
    #[error("Failed to init cli editor")]
    InitCliEditor(#[source] ReadlineError),
    #[error("Failed to read line from cli")]
    ReadingLine(#[source] ReadlineError),
    #[error("Error while handling command")]
    Command(#[from] client::Error),
}

/// # Safety
///
/// Only call when it is safe to call fork()
pub unsafe fn entrypoint(args: Vec<String>) -> ExitCode {
    match unsafe { run(args) } {
        Daemonized::InsideDaemon(Ok(())) | Daemonized::OutsideDaemon(Ok(())) => ExitCode::SUCCESS,
        Daemonized::InsideDaemon(Err(err)) => {
            match &err {
                Error::StartingDaemon(daemon::Error::RedirectingDaemonIO(err)) => {
                    logging::log_with_msg(
                        "Daemon failed to redirect it's IO",
                        &err,
                        LogLevel::Stderr,
                    )
                }
                _ => logging::log(&err, LogLevel::Error),
            }
            ExitCode::FAILURE
        }
        Daemonized::OutsideDaemon(Err(err)) => {
            logging::log(&err, LogLevel::Stderr);
            ExitCode::FAILURE
        }
    }
}

/// # Safety
///
/// Only call when it is safe to call fork()
unsafe fn run(mut args: Vec<String>) -> Daemonized<(), Error, (), Error> {
    let mut maybe_rl = None;

    let start_mode = StartMode::from_args(&mut args);

    let is_daemon_started = if start_mode.is_force() {
        false
    } else {
        try_daemonized_outside!(is_daemon_started())
    };

    if !is_daemon_started {
        let mut rl =
            try_daemonized_outside!(rustyline::DefaultEditor::new().map_err(Error::InitCliEditor));

        if !matches!(start_mode, StartMode::Force | StartMode::Acknowledge)
            && !should_start_daemon(&mut rl)
        {
            return Daemonized::OutsideDaemon(Ok(()));
        }

        maybe_rl = Some(rl);

        match try_daemonized!(unsafe { daemon::run(start_mode) }) {
            DaemonizedOk::InsideDaemon(()) => return Daemonized::InsideDaemon(Ok(())),
            DaemonizedOk::OutsideDaemon(_pid) => {
                // waiting a bit for the daemon to start before starting client
                sleep(Duration::from_millis(400));
            }
        }
    }

    Daemonized::OutsideDaemon(client::run(maybe_rl, args))
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
