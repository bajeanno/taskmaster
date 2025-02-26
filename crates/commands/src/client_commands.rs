use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClientCommands {
    SuccessfulConnection,
    FailedToParseFrame,

    TaskList(String),
}
