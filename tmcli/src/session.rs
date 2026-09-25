use std::io;

pub struct Session {}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConnectError {
    #[error("Failed to connect to Taskmaster server")]
    ConnectionFailure(#[from] io::Error),
}

impl Session {
    pub async fn new() -> Result<Self, ConnectError> {
        Ok(Self {})
    }
}
