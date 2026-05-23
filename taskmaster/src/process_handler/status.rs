use std::{fmt::Debug, process::ExitStatus};

use crate::process_handler::RoutineSpawnError;

#[allow(dead_code)] //TODO: remove that
#[derive(Debug)]
#[cfg_attr(test, derive(Clone))]
pub struct NominativeStatus {
    pub process_name: String,
    pub status: Status,
}

#[allow(dead_code)] //TODO: Remove that
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub enum Status {
    Starting,
    Running,
    ErrorDuringStartup { exit_code: u8 },
    FailedToStartProcess(String),
    Exited(ExitStatus),
    FailedToSpawnRoutine(RoutineSpawnError),
}

impl Status {
    pub fn is_running(&self) -> bool {
        matches!(self, Status::Running | Status::Starting)
    }
}
