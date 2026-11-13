use std::{
    fmt::{Debug, Display},
    os::unix::process::ExitStatusExt,
    process::ExitStatus,
};

use tokio::sync::mpsc::UnboundedSender;

use crate::daemon::process::ProcessId;

#[allow(dead_code)] //TODO: remove that
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct NominativeStatus {
    pub process_id: ProcessId,
    pub status: Status,
}

impl Display for NominativeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.process_id, self.status)
    }
}

#[allow(dead_code)] //TODO: Remove that
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub enum Status {
    #[default]
    NotRunning,
    RoutineStarting,
    Starting,
    Running,
    FailedToStartProcess(String),
    #[serde(
        serialize_with = "serialize_exit_status",
        deserialize_with = "deserialize_exit_status"
    )]
    ErrorDuringStartup(ExitStatus),
    #[serde(
        serialize_with = "serialize_exit_status",
        deserialize_with = "deserialize_exit_status"
    )]
    Exited(ExitStatus),
    NotRestarting {
        instance_id: usize,
    },
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::NotRunning => write!(f, "Not running"),
            Status::RoutineStarting => write!(f, "Routine starting"),
            Status::Starting => write!(f, "Starting"),
            Status::Running => write!(f, "Running"),
            Status::FailedToStartProcess(_) => write!(f, "Failed to start process"),
            Status::ErrorDuringStartup(exit_status) => {
                write!(f, "Exited during startup with exit code {}", exit_status)
            }
            Status::Exited(exit_status) => write!(f, "Exited with exit code {}", exit_status),
            Status::NotRestarting { .. } => write!(f, "Not restarting"),
        }
    }
}

fn serialize_exit_status<S>(exit_status: &ExitStatus, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_i32(exit_status.into_raw())
}

fn deserialize_exit_status<'de, D>(deserializer: D) -> Result<ExitStatus, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    Ok(ExitStatus::from_raw(
        i32::deserialize(deserializer).map_err(|err| {
            serde::de::Error::custom(format!("Failed to parse ExitStatus: {err}"))
        })?,
    ))
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
