#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    #[error("Error opening taskmaster config file: {file}")]
    OpeningFile {
        file: String,
        source: std::io::Error,
    },
    #[error("Error parsing taskmaster config file: {file}")]
    InvalidConfig {
        file: String,
        source: serde_yaml::Error,
    },
    #[error("Error creating default config file")]
    CreatingDefaultConfigFileError(#[from] CreatingDefaultConfigFileError),
}

#[derive(thiserror::Error, Debug)]
pub enum CreatingDefaultConfigFileError {
    #[error("Failed to create file: {file}")]
    UnableToCreate {
        file: String,
        source: std::io::Error,
    },
    #[error("Failed to write to file: {file}")]
    UnableToWrite {
        file: String,
        source: serde_yaml::Error,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("Empty command")]
    EmptyCommand,
    #[error("{0}")]
    SplitError(#[from] shell_words::ParseError),
}
