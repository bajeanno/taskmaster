use std::{fmt::Debug, process::ExitStatus};

#[allow(dead_code)] //TODO: remove that
#[derive(Debug)]
pub struct NominativeStatus {
    pub process_name: String,
    pub status: Status,
}

#[allow(dead_code)] //TODO: Remove that
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Starting,
    Running,
    ErrorDuringStartup { exit_code: u8 },
    FailedToSpawn(String),
    Exited(ExitStatus),
}
