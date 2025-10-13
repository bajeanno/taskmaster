pub mod shell {
    use std::fmt;

    use crate::client::{parsing::parse_command, send_command};

    pub enum ShellError {
        FailedToParse(String),
        BadCommand,
        ConnectionError,
    }

    impl fmt::Display for ShellError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::FailedToParse(cmd) => write!(f, "Failed to parse command {cmd}"),
                Self::ConnectionError => write!(f, "Failed to connect to taskmaster daemon"),
                Self::BadCommand => write!(f, "bad command"),
            }
        }
    }

    pub async fn run() -> Result<(), ShellError> {
        loop {
            let mut prompt = String::new();
            std::io::stdin().read_line(&mut prompt).map_err(|_| ShellError::BadCommand)?;
            let vec: Vec<String> = prompt.split(' ').map(|item| item.to_string()).collect();
            let Some(cmd) = parse_command(vec.into_iter()).map_err(|_| ShellError::FailedToParse(prompt))? else {
                return Ok(());
            };
            send_command(cmd).await.map_err(|_| ShellError::ConnectionError)?;
        }
    }
}

pub use shell::run;
