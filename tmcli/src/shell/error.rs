use thiserror::Error;
use tokio::io;

#[derive(Error, Debug)]
pub enum ShellError {
    #[error("Failed to read standard input: {0}")]
    ReadingStdin(io::Error),
}
