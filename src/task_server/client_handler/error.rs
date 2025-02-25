use std::fmt::Display;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    #[allow(dead_code)]
    FailedToReadFrameFromClient {
        client_id: u64,
        error: connection::Error,
    },

    #[allow(dead_code)]
    FailedToWriteFrameFromClient {
        client_id: u64,
        error: connection::Error,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl core::error::Error for Error {}
