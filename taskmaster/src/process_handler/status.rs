use std::process::ExitCode;

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum Status {
    NotSpawned,
    Starting,
    Running,
    FailedToStart(ExitCode),
    Stopped,
}
