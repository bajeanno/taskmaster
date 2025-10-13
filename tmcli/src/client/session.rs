use commands::{ClientCommand, ServerCommand};
use connection::Connection;
use std::fmt::Display;
use std::io;
use tokio::net::TcpStream;

pub struct Session {
    pub _stream: Connection<TcpStream, ClientCommand, ServerCommand>,
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
            _stream: Connection::new(socket, 1024),
        })
    }
}
