mod handle;
mod process;
mod routine;
mod state_machine;

use crate::NominativeStatus;
use process::Process;
use tokio::sync::oneshot;

#[derive(Debug)]
pub enum ServerCommandError {
    NoSuchTask(String),
}
pub enum TaskManagerCommand {
    ListTasks(oneshot::Sender<Vec<NominativeStatus>>),
    Start { task_name: String },
    Restart { task_name: String },
    Stop { task_name: String },
}
