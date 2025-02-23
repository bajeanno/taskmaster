use std::{fmt::Display, io};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    FailedToFork {
        os_error: io::Error,
    },

    FailedToOpenFile {
        file_path: String,
        redirected_io: &'static str,
        err: io::Error,
    },

    FailedToRedirectFileUsingDup2 {
        file_path: String,
        redirected_io: &'static str,
        os_error: io::Error,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl core::error::Error for Error {}
