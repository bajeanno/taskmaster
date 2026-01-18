use std::process::ExitStatus;

#[allow(dead_code)]
#[derive(Clone)]
pub enum Status {
    NotSpawned,
    Starting,
    Running,
    FailedToStart {
        error_message: String,
        exit_code: Option<u8>,
    },
    Exited(ExitStatus),
}
