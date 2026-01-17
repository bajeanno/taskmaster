mod parsed_program;
pub mod program;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error(
        "Error opening taskmaster config file: {0}\n\
     Consider making a reload request after creating one"
    )]
    OpeningFile(#[from] std::io::Error),
    #[error(
        "Error parsing taskmaster config file: {0}\n\
     Consider making a reload request after fixing the issue"
    )]
    InvalidYaml(#[from] serde_yaml::Error),
    #[error(
        "Error parsing taskmaster config file: invalid umask for program {0}\n\
     Consider making a reload request after fixing the issue"
    )]
    InvalidUmask(String),
    #[error(
        "Error parsing taskmaster config file: invalid stopsignal {signal} for program \
     {program_name}\n\
     Consider making a reload request after fixing the issue"
    )]
    InvalidSignal {
        signal: String,
        program_name: String,
    },
    #[error(
        "Error parsing taskmaster config file: Empty command for program {0}\n\
     Consider making a reload request after fixing the issue"
    )]
    EmptyCommand(String),
    #[error(
        "Error parsing taskmaster config file: {0}\n\
     Consider making a reload request after fixing the issue"
    )]
    CommandError(#[from] shell_words::ParseError),
}
