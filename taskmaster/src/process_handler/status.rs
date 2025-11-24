use std::process::ExitCode;

#[allow(dead_code)]
#[derive(Clone)]
pub enum Status {
    NotSpawned,
    Starting,
    Running,
    FailedToStart(String),
    Exited(ExitCode),
}
