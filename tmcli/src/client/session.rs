use std::fmt::Display;
use tokio::net::TcpStream;
use std::io;
use connection::Connection;
use commands::{ClientCommand, ServerCommand};

use crate::client::ServerError;

pub struct Session {
    stream: Connection<TcpStream, ClientCommand, ServerCommand>,
}

#[derive(Debug)]
pub enum ConnectError {
    NotRunning,
    ConnectionFailure,
}

impl From<io::Error> for ConnectError {
    fn from(_value: io::Error) -> Self {
        return ConnectError::NotRunning;
    }
}

impl Display for ConnectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotRunning => write!(f, "Taskmaster server is not running"),
            Self::ConnectionFailure => write!(f, "Failed to connect to Taskmaster server"),
        }
    }
}

impl Session {
    pub async fn new() -> Result<Self, ConnectError> {
        let socket = TcpStream::connect("localhost:4444")
            .await
            .map_err(|_| ConnectError::ConnectionFailure)?;
        Ok(Self {
            stream: Connection::new(socket, 1024),
        })
    }
    pub async fn request(&mut self, command: ServerCommand) -> Result<(), ServerError> {
        self.stream.write_frame(&command).await?;
        match self.stream.read_frame().await {
            Ok(Some(_)) => Ok(()),
            Ok(None) => Ok(()),
            Err(err) => Err(ServerError::RequestError(err)),
        }
    }
}
