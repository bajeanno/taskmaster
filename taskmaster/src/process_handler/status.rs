use std::{fmt::Debug, process::ExitStatus};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Status {
    Starting,
    Running,
    ErrorDuringStartup { exit_code: u8 },
    FailedToSpawn(String),
    Exited(ExitStatus),
}
