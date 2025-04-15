use std::fmt;


pub enum ShellError {
    FailedToParse(String),
    BadCommand,
    ConnectionError,
}

impl ShellError {
    pub fn get_code(&self) -> u8 {
        match self {
            Self::FailedToParse(_) => 1,
            Self::ConnectionError => 2,
            Self::BadCommand => 3,
        }
    }
}

impl fmt::Display for ShellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FailedToParse(cmd) => write!(f, "Failed to parse command {cmd}"),
            Self::ConnectionError => write!(f, "Failed to connect to taskmaster daemon"),
            _ => write!(f, "bad command")
        }
    }
}

pub fn run() -> Result<(), ShellError> {
    loop {
        break;
    }
    Ok(())
}
