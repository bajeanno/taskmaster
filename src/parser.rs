pub mod program;
use program::{Config, ParsedConfig};
pub use program::Program;
use std::{fmt::Display, fs::File};

#[derive(Debug)]
pub enum ParseError {
    OpenError(std::io::Error),
    YamlError(serde_yaml::Error),
    SignalError(String, String),
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::OpenError(err) => write!(f, "Error opening taskmaster config file: {err}\nConsider making a reload request after creating one"),
            ParseError::YamlError(err) => write!(f, "Error parsing taskmaster config file: {err}\nConsider making a reload request after fixing the issue"),
            ParseError::SignalError(sig, prog_name) => write!(f, "Error parsing taskmaster config file: invalid stopsignal {sig} for program {prog_name}\nConsider making a reload request after fixing the issue"),
        }
    }
}

impl std::error::Error for ParseError {}

pub struct Parser {}

impl Parser {
    pub fn parse(filename: &str) -> Result<Vec<Program>, ParseError> {
        let file = File::open(filename).map_err(ParseError::OpenError)?;
        let parsed_config = ParsedConfig::new(file).map_err(ParseError::YamlError)?;
        let config = Config::from(parsed_config);
        for program in config.programs.iter() {
            program.check_signal()?;
        }
        Ok(config.programs)
    }

}
