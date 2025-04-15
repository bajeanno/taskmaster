use std::fmt::Display;
use std::net::TcpStream;
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
    NotRunning(Error),
}

impl From<Error> for ConnectError {
    fn from(value: Error) -> Self {
        return ConnectError::NotRunning(value);
    }
}

impl Display for ConnectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotRunning(_) => write!(f, "Taskmaster server is not running"),
        }
    }
}

impl Session {
    pub fn new() -> Result<Self, ConnectError> {
        Ok(Self {
            stream: TcpStream::connect("127.0.0.1:4444")?,
        })
    }
    pub fn request(&self, ServerCommand) -> Result<(), SessionError>{
        let ctx = Context::new_dbus(LE, 0);
        Ok(())
    }
}
