use std::{fmt::Debug, os::unix::process::ExitStatusExt, process::ExitStatus};

use tokio::sync::mpsc::UnboundedSender;

use crate::daemon::process::ProcessId;

#[allow(dead_code)] //TODO: remove that
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct NominativeStatus {
    pub process_id: ProcessId,
    pub status: Status,
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
