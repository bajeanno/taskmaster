use crate::rpc::StartServerError;

use crate::daemon::{claim_pid::ClaimError, config_state, signal_handling::SignalError};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Daemon failed to redirect it's IO")]
    RedirectingDaemonIO(#[from] daemonize::RedirectIOError),

    #[error("Failed to daemonize process")]
    Fork(#[from] daemonize::ForkFailed),

    #[error("Failed to initialize config state")]
    ConfigStateInit(#[from] config_state::InitFileError),

    #[error("Failed to start daemon server")]
    TaskServerFailure(#[from] StartServerError),

    #[error("Failed to claim taskmaster daemon instance")]
    Claim(#[from] ClaimError),

    #[error("Failed to initiate signal handler")]
    Signal(#[from] SignalError),
}
