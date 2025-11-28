mod parsed_program;
pub mod program;
use std::fmt::Display;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    OpenningFile(#[from] std::io::Error),
    InvalidYaml(serde_yaml::Error),
    InvalidUmask(String, String),
    InvalidSignal(String, String),
    EmptyCommand(String),
    CommandError(#[from] shell_words::ParseError),
}

impl From<serde_yaml::Error> for ParseError {
    fn from(err: serde_yaml::Error) -> ParseError {
        ParseError::InvalidYaml(err)
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::OpenningFile(err) => write!(
                f,
                "Error opening taskmaster config file: {err}\n\
                 Consider making a reload request after creating one"
            ),
            ParseError::InvalidYaml(err) => write!(
                f,
                "Error parsing taskmaster config file: {err}\n\
                 Consider making a reload request after fixing the issue"
            ),
            ParseError::InvalidSignal(sig, prog_name) => write!(
                f,
                "Error parsing taskmaster config file: invalid stopsignal {sig} for program \
                 {prog_name}\n\
                 Consider making a reload request after fixing the issue"
            ),
            ParseError::InvalidUmask(sig, prog_name) => write!(
                f,
                "Error parsing taskmaster config file: {sig} for program {prog_name}\n\
                 Consider making a reload request after fixing the issue"
            ),
            ParseError::EmptyCommand(prog_name) => write!(
                f,
                "Error parsing taskmaster config file: Empty command for program {prog_name}\n\
                 Consider making a reload request after fixing the issue"
            ),
            ParseError::CommandError(error) => write!(
                f,
                "Error parsing taskmaster config file: {error}\n\
                 Consider making a reload request after fixing the issue"
            ),
        }
    }
}
