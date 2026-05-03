use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServerCommand {
    ListTasks,
    Stop { task_name: String },
    Restart { task_name: String },
    Start { task_name: String },
}
