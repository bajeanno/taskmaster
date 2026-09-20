use std::{fmt::Debug, process::ExitStatus};

use tokio::sync::mpsc::UnboundedSender;

use crate::{process::ProcessId, process_handler::RoutineSpawnError};

#[allow(dead_code)] //TODO: remove that
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct NominativeStatus {
    pub process_id: ProcessId,
    pub status: Status,
}

#[allow(dead_code)] //TODO: Remove that
#[derive(Debug, Clone, Default)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub enum Status {
    #[default]
    NotRunning,
    RoutineStarting,
    Starting,
    Running,
    FailedToStartProcess(String),
    ErrorDuringStartup(ExitStatus),
    Exited(ExitStatus),
    FailedToSpawnRoutine(RoutineSpawnError),
    NotRestarting {
        instance_id: usize,
    },
}

impl Status {
    pub fn is_running(&self) -> bool {
        matches!(self, Status::Starting | Status::Running)
    }
}

pub struct StatusSender {
    sender: UnboundedSender<NominativeStatus>,
    process_id: ProcessId,
}

impl StatusSender {
    pub fn new(sender: UnboundedSender<NominativeStatus>, process_id: ProcessId) -> Self {
        Self { sender, process_id }
    }

    pub fn send_new_status_to_task_manager(&mut self, status: Status) {
        self.sender
            .send(NominativeStatus {
                process_id: self.process_id.clone(),
                status,
            })
            .expect("Receiver was dropped");
    }
}
