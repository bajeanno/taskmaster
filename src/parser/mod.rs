mod parsed_program;
pub mod program;
use std::fmt::Display;

#[derive(Debug)]
pub enum ParseError {
    OpenError(std::io::Error),
    InvalidYaml(serde_yaml::Error),
    InvalidUmask(String, String),
    InvalidSignal(String, String),
}

impl From<serde_yaml::Error> for ParseError {
    fn from(err: serde_yaml::Error) -> ParseError {
        ParseError::InvalidYaml(err)
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::OpenError(err) => write!(f, "Error opening taskmaster config file: {err}\nConsider making a reload request after creating one"),
            ParseError::InvalidYaml(err) => write!(f, "Error parsing taskmaster config file: {err}\nConsider making a reload request after fixing the issue"),
            ParseError::InvalidSignal(sig, prog_name) => write!(f, "Error parsing taskmaster config file: invalid stopsignal {sig} for program {prog_name}\nConsider making a reload request after fixing the issue"),
            ParseError::InvalidUmask(sig, prog_name) => write!(f, "Error parsing taskmaster config file: {sig} for program {prog_name}\nConsider making a reload request after fixing the issue"),
        }
    }
}

impl std::error::Error for ParseError {}
