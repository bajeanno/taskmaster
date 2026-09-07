mod handle;
mod process;
mod process_registry;
mod routine;

#[cfg(test)]
mod tests;

use crate::process_handler::NominativeStatus;
use routine::Client;
use tokio::sync::oneshot;

#[derive(Debug)]
// TODO: use thiserror
pub enum ServerCommandError {
    NoSuchProgram(String),
    FailedToLoadNewConfig(String),
}

pub enum TaskManagerCommand {
    ListProcesses(oneshot::Sender<Vec<Vec<NominativeStatus>>>),
    Reload {
        config_file_name: String,
    },
    StartProgram {
        program_name: String,
    },
    RestartProgram {
        program_name: String,
    },
    StopProgram {
        program_name: String,
    },
    SubscribeToProgramEvents {
        program_name: String,
        client: Client,
    },
    UnsubscribeToProgramEvents {
        program_name: String,
        client: Client,
    },
    StopAllProcesses,
    Exit,
}

fn split_process_name(mut process_name: String) -> Option<(String, usize)> {
    let dash_index = process_name.rfind('-')?;
    let tmp = process_name.split_off(dash_index);
    let id: usize = tmp[1..].parse().ok()?;
    let program_name = process_name;
    Some((program_name, id))
}

#[test]
fn test_split_process_name() {
    let process_name = "taskmaster_test_task-0".to_string();
    assert_eq!(
        split_process_name(process_name),
        Some(("taskmaster_test_task".to_string(), 0))
    );
}
