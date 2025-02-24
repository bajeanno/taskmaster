use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientCommands {
    SuccessfulConnection,
    FailedToParseFrame,
    Test, // TODO delete me
}
