use std::{io::Error, net::{SocketAddrV4, TcpStream, ToSocketAddrs}};

enum SessionError {
    ConnectError(String),
}

impl From for SessionError {
    fn from(value: T) -> Self {
        Self::ConnectError(match value {
            Error => "couldn't connect to server",
        })
    }
}

pub struct Session {
    addr: SocketAddrV4,
    stream: TcpStream,
}

impl Session {
    pub fn new(address: String, port: String) -> Result<Self, SessionError> {
        return Ok(Self {
            stream: TcpStream::connect().map_err(|err| {
                SessionError::ConnectError(err.to_string());
            })?
        });
    }
}
