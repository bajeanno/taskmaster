pub mod program;
pub use program::Program;
use program::{Config, ParsedConfig};
use std::{fmt::Display, fs::File};

#[derive(Debug)]
pub enum ParseError {
    FailedToOpenFile(std::io::Error),
    InvalidYaml(serde_yaml::Error),
    InvalidSignal(String, String),
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::FailedToOpenFile(err) => write!(f, "Error opening taskmaster config file: {err}\nConsider making a reload request after creating one"),
            ParseError::InvalidYaml(err) => write!(f, "Error parsing taskmaster config file: {err}\nConsider making a reload request after fixing the issue"),
            ParseError::InvalidSignal(sig, prog_name) => write!(f, "Error parsing taskmaster config file: invalid stopsignal {sig} for program {prog_name}\nConsider making a reload request after fixing the issue"),
        }
    }
}

impl std::error::Error for ParseError {}

pub struct Parser {}

impl Parser {
    pub fn parse(filename: &str) -> Result<Vec<Program>, ParseError> {
        let file = File::open(filename).map_err(ParseError::FailedToOpenFile)?;
        let parsed_config = ParsedConfig::new(file).map_err(ParseError::InvalidYaml)?;
        let config = Config::from(parsed_config);
        for program in config.programs.iter() {
            program.check_signal()?;
        }
        Ok(config.programs)
    }
}
