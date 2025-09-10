use std::fmt::Display;
use tokio::net::TcpStream;
use std::io::Error;
use zvariant::serialized::Context;

pub struct Session {
    stream: TcpStream,
}

pub enum SessionError {
    ConnectError(ConnectError),
    RequestError,
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
    pub async fn new() -> Result<Self, SessionError> {
        let socket = TcpStream::connect("localhost:4444")
            .await
            .map_err(|err| SessionError::ConnectError(ConnectError::ConnectionFailure(err)))?;
        Ok(Self {
            stream: Connection::new(socket, 1024),
        })
        // todo!("finish connection");
    }
    pub async fn request(&self, _command: ServerCommand) -> Result<(), SessionError> {
        todo!("make request")
    }
}
