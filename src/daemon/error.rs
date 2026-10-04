use std::num::ParseIntError;

use crate::rpc::StartServerError;

use crate::daemon::{config_state, signal_handling::SigActionError};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to daemonize process")]
    FailedToDaemonize(#[from] daemonize::Error),

    #[error("Failed to initialize config state")]
    ConfigStateInit(#[from] config_state::InitFileError),

    #[error("Failed to start daemon server")]
    TaskServerFailure(#[from] StartServerError),

    #[error("Failed to claim taskmaster daemon instance")]
    Claim(#[from] ClaimError),

    #[error("Failed to initiate signal handler")]
    Signal(#[from] SigActionError),
}

#[derive(Debug, thiserror::Error)]
pub enum ClaimError {
    #[error("Failed to open pid file")]
    OpenFile(#[source] std::io::Error),
    #[error("Failed to lock pid file")]
    Flock(#[source] std::io::Error),
    #[error("Failed to read pid file")]
    ReadFile(#[source] std::io::Error),
    #[error("Failed to write to pid file")]
    WriteFile(#[source] std::io::Error),
    #[error("Failed to lock pid file")]
    Flock,
    #[error("Failed to parse pid file content")]
    Parse(#[source] ParseIntError),
    #[error(
        "The taskmaster PID file contains a pid meaning another instance of the daemon is \
        currently running.\n\
        If taskmaster was killed unexpectedly last time, or if you're sure taskmaster is \
        not running on this machine, feel free to delete /var/run/taskmaster.d/taskmaster.pid \
        and restart it"
    )]
    OtherInstanceRunning,
}
