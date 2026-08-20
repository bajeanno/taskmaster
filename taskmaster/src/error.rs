use std::num::ParseIntError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to parse port number from input: '{input}': {error}")]
    PortArgumentIsNotAnInteger {
        input: String,
        error: std::num::ParseIntError,
    },

    #[allow(dead_code)] //TODO: remove that
    #[error("")] // TODO: write error message
    FailedToDaemonize(#[from] daemonize::Error),

    #[allow(dead_code)] //TODO: remove that
    #[error("")] // TODO: write error message
    OpenError(#[from] std::io::Error),

    #[allow(dead_code)] //TODO: remove that
    #[error("Failed to parse pid from PID file: {0}")]
    PidParseError(#[from] ParseIntError),

    #[allow(dead_code)] //TODO: remove that
    #[error("")] // TODO: write error message
    TaskServerFailure,
}
