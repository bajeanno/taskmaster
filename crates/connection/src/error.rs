use std::{error::Error, fmt::Display, io};

pub type FrameEncodeError = rmp_serde::encode::Error;
pub type FrameDecodeError = rmp_serde::decode::Error;

#[derive(Debug)]
pub enum ConnectionError {
    ConnectionReset,
    FailedToDecodeFrame(FrameDecodeError),
    FailedToEncodeFrame(FrameEncodeError),
    FailedToReadFromStream(io::Error),
    FailedToWriteToStream(io::Error),
}

impl Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for ConnectionError {}
