use serializable_error::SerializableError;

#[derive(thiserror::Error, Debug, serde::Serialize, serde::Deserialize)]
pub enum ParseError {
    #[error("Error opening taskmaster config file: {file}")]
    OpeningFile {
        file: String,
        #[source]
        io_error: SerializableError,
    },
    #[error("Error parsing taskmaster config file: {file}")]
    InvalidConfig {
        file: String,
        #[source]
        serde_yaml_error: SerializableError,
    },
    #[error("Error creating default config file")]
    CreatingDefaultConfigFileError(#[from] CreatingDefaultConfigFileError),
}

#[derive(thiserror::Error, Debug, serde::Serialize, serde::Deserialize)]
pub enum CreatingDefaultConfigFileError {
    #[error("Failed to create file: {file}")]
    UnableToCreate {
        file: String,
        #[source]
        io_error: SerializableError,
    },
    #[error("Failed to write to file: {file}")]
    UnableToWrite {
        file: String,
        #[source]
        serde_yaml_error: SerializableError,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("Empty command")]
    EmptyCommand,
    #[error("{0}")]
    SplitError(#[from] shell_words::ParseError),
}
