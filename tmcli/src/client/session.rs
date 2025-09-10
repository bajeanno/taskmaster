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
    NotRunning(io::Error),
    ConnectionFailure(io::Error),
}

impl From<io::Error> for ConnectError {
    fn from(value: io::Error) -> Self {
        return ConnectError::NotRunning(value);
    }
}

impl Display for ConnectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotRunning(_) => write!(f, "Taskmaster server is not running"),
            Self::ConnectionFailure(_) => write!(f, "Failed to connect to Taskmaster server"),
        }
    }
}

impl Session {
    pub async fn new() -> Result<Self, ConnectError> {
        let socket = TcpStream::connect("localhost:4444")
            .await
            .map_err(|err| ConnectError::ConnectionFailure(err))?;
        Ok(Self {
            stream: Connection::new(socket, 1024),
        })
    }
    pub async fn request(&mut self, command: ServerCommand) -> Result<(), ServerError> {
        self.stream.write_frame(&command).await;
        let server_return = self.stream.read_frame();
        match server_return {
            Ok(Some(response)) => match response {
                ServerCommand::Error(err_msg) => Err(ServerError::RequestError),
                _ => Ok(()),
            },
            Ok(None) => Err(ServerError::RequestError),
            Err(_) => Err(ServerError::RequestError),
        }
    }
}
