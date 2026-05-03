mod handle;
mod process;
mod routine;
mod tests;

use crate::NominativeStatus;
use process::Process;
use routine::Client;
use tokio::sync::oneshot;

#[derive(Debug)]
pub enum ServerCommandError {
    NoSuchTask(String),
}
pub enum TaskManagerCommand {
    ListTasks(oneshot::Sender<Vec<NominativeStatus>>),
    StartTask { task_name: String },
    RestartTask { task_name: String },
    StopTask { task_name: String },
    AddClient { task_name: String, client: Client },
    DeleteClient { task_name: String, client: Client },
    StopAll,
    Exit,
}
