use std::num::ParseIntError;

use shared_code::rpc::StartServerError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to daemonize process")]
    FailedToDaemonize(#[from] daemonize::Error),

    #[error("Failed to start daemon server")]
    TaskServerFailure(#[from] StartServerError),

    #[error("Failed to claim taskmaster daemon instance")]
    Pid(#[from] PidError),
}

#[derive(Debug, thiserror::Error)]
pub enum PidError {
    #[error("Failed to open pid file")]
    OpenFile(#[source] std::io::Error),
    #[error("Failed to read pid file")]
    ReadFile(#[source] std::io::Error),
    #[error("Failed to write to pid file")]
    WriteFile(#[source] std::io::Error),
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
