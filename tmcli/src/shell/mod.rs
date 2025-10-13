use std::fmt;

use crate::client::{
    parsing::parse_command,
    send_command,
    session::{ConnectError, Session},
};

#[derive(Debug)]
pub enum ShellError {
    BadCommand,
    ConnectionError,
}

impl From<ConnectError> for ShellError {
    fn from(_: ConnectError) -> Self {
        Self::ConnectionError
    }
}

impl fmt::Display for ShellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionError => write!(f, "Failed to connect to taskmaster daemon"),
            Self::BadCommand => write!(f, "bad command"),
        }
    }
}

pub async fn run() -> Result<(), ShellError> {
    let session = Session::new().await.map_err(|_| ConnectError::NotRunning)?;
    loop {
        let mut prompt = String::new();
        std::io::stdin()
            .read_line(&mut prompt)
            .map_err(|_| ShellError::BadCommand)?;
        let vec: Vec<String> = prompt.split(' ').map(|item| item.to_string()).collect();
        let Ok(cmd) = parse_command(vec.into_iter()) else {
            continue;
        };
        let Some(cmd) = cmd else {
            return Ok(());
        };
        send_command(cmd, &session)
            .await
            .map_err(|_| ShellError::ConnectionError)?;
    }
}
