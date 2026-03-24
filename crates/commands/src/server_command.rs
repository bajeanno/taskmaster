use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServerCommand {
    ListTasks,
    Stop { target: String },
    Restart { target: String },
    Start { target: String },
}
