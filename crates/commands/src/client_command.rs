use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClientCommand {
    SuccessfulConnection,
    FailedToParseFrame,

    TaskList(String),
}
