use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServerCommand {
    ListTasks,
    Stop { process_name: String },
    Restart { process_name: String },
    Start { process_name: String },
}
