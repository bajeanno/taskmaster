use std::{fmt::Debug, process::ExitStatus};

use tokio::sync::mpsc::UnboundedSender;

use crate::process_handler::RoutineSpawnError;

#[allow(dead_code)] //TODO: remove that
#[derive(Debug, Clone)]
pub struct NominativeStatus {
    pub process_name: String,
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
        instance_id: u32,
    },
}

impl Status {
    pub fn is_running(&self) -> bool {
        matches!(self, Status::Starting | Status::Running)
    }
}

pub struct StatusSender {
    sender: UnboundedSender<NominativeStatus>,
    process_name: String,
}

impl StatusSender {
    pub fn new(sender: UnboundedSender<NominativeStatus>, process_name: String) -> Self {
        Self {
            sender,
            process_name,
        }
    }

    pub fn send_new_status_to_task_manager(&mut self, status: Status) {
        let process_name = self.process_name.clone();
        self.sender
            .send(NominativeStatus {
                process_name,
                status,
            })
            .expect("Receiver was dropped");
    }
}
