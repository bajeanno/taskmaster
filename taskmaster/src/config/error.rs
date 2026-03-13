use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error(
        "Error opening taskmaster config file: {file}: {error}\n\
      Consider making a reload request after fixing the issue"
    )]
    OpeningFile {
        file: String,
        #[source]
        error: std::io::Error,
    },
    #[error(
        "Error parsing taskmaster config file: {file}: {error}\n\
      Consider making a reload request after fixing the issue"
    )]
    InvalidConfig {
        file: String,
        #[source]
        error: serde_yaml::Error,
    },
}

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("Empty command")]
    EmptyCommand,
    #[error("{0}")]
    SplitError(#[from] shell_words::ParseError),
}
