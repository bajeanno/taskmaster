use std::{fmt::Display, io};

pub type FrameEncodeError = rmp_serde::encode::Error;
pub type FrameDecodeError = rmp_serde::decode::Error;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    ConnectionReset,
    FailedToDecodeFrame(FrameDecodeError),
    FailedToEncodeFrame(FrameEncodeError),
    FailedToReadFromStream(io::Error),
    FailedToWriteToStream(io::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl core::error::Error for Error {}
