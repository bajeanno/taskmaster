use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClientCommand {
    SuccessfulConnection,

    /// The connection will be closed after sending this command
    FailedToParseFrame,

    TaskList(Vec<String>),
}
