use crate::client::{
    parsing::parse_command,
    send_command,
    session::{ConnectError, Session},
};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ShellError {
    #[error("Bad command")]
    BadCommand,
    #[error("Failed to connect to taskmaster daemon")]
    ConnectionError,
}

impl From<ConnectError> for ShellError {
    fn from(_: ConnectError) -> Self {
        Self::ConnectionError
    }
}

pub async fn run() -> Result<(), ShellError> {
    let session = Session::new()
        .await
        .map_err(|_| ConnectError::ConnectionFailure)?;
    loop {
        let mut prompt = String::new();
        std::io::stdin()
            .read_line(&mut prompt)
            .map_err(|_| ShellError::BadCommand)?;
        let iter = prompt.split(' ').map(|item| item.to_string());
        let cmd = match parse_command(iter) {
            Ok(cmd) => {
                let Some(cmd) = cmd else {
                    return Ok(());
                };
                cmd
            }
            Err(e) => {
                eprintln!("{e}");
                continue;
            }
        };
        send_command(cmd, &session)
            .await
            .map_err(|_| ConnectError::ConnectionFailure)?;
    }
}
