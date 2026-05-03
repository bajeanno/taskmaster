use std::{fmt::Debug, process::ExitStatus};

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Starting,
    Running,
    ErrorDuringStartup { exit_code: u8 },
    FailedToSpawn(String),
    Exited(ExitStatus),
}
