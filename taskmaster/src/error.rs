use std::num::ParseIntError;

use shared_code::rpc::StartServerError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to daemonize process")]
    FailedToDaemonize(daemonize::Error),

    #[error("Failed to start daemon server")]
    TaskServerFailure(#[from] StartServerError),

    #[error("Failed to claim taskmaster daemon instance")]
    Pid(#[from] PidError),
}

#[derive(Debug, Error)]
pub enum PidError {
    #[error("Failed to open pid file: {0}")]
    OpenFile(std::io::Error),
    #[error("Failed to read pid file: {0}")]
    ReadFile(std::io::Error),
    #[error("Failed to write to pid file: {0}")]
    WriteFile(std::io::Error),
    #[error("Failed to parse pid file content: {0}")]
    Parse(ParseIntError),
    #[error(
        "The taskmaster PID file is containing a pid meaning another instance of the server is \
        running.
If taskmaster was killed unexpectedly last time, or if you're sure taskmaster is
not running on this machine, feel free to delete the taskmaster pidfile under
/var/run/taskmaster.d/taskmaster.pid and restart it"
    )]
    OtherInstanceRunning,
}

impl From<daemonize::Error> for Error {
    fn from(error: daemonize::Error) -> Self {
        Self::FailedToDaemonize(error)
    }
}
