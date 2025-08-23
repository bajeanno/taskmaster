use std::{fmt::Display, io};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    #[allow(dead_code)]
    BindTcpListener { addr: String, error: io::Error },
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl core::error::Error for Error {}
