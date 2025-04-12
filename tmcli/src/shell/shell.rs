use std::process::ExitCode;

pub enum ShellError {
    FailedToParse,
    BadCommand,
    ConnectionError,
}

impl ShellError {
    pub fn get_code(&self) -> u8 {
        match self {
            Self::FailedToParse => 1,
            Self::ConnectionError => 2,
            Self::BadCommand => 3,
        }
    }
}

pub fn run() -> Result<ExitCode, ShellError> {
    loop {
        break;
    }
    Ok(ExitCode::SUCCESS)
}
