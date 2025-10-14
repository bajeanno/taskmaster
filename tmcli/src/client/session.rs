use commands::{ClientCommand, ServerCommand};
use connection::Connection;
use std::io;
use tokio::net::TcpStream;

pub struct Session {
    pub _stream: Connection<TcpStream, ClientCommand, ServerCommand>,
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConnectError {
    #[error("Failed to connect to Taskmaster server")]
    ConnectionFailure,
}

impl From<io::Error> for ConnectError {
    fn from(_value: io::Error) -> Self {
        ConnectError::ConnectionFailure
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
