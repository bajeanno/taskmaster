pub mod program;
use program::Config;
pub use program::Program;
use std::{fs::File, collections::HashMap, fmt::Display};

#[derive(Debug)]
pub enum ParseError {
    OpenError(std::io::Error),
    YamlError(serde_yaml::Error),
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::OpenError(err) => write!(f, "Error opening taskmaster config file: {err}\nConsider making a reload request after creating one"),
            ParseError::YamlError(err) => write!(f, "Error parsing taskmaster config file: {err}\nConsider making a reload request after fixing the issue"),
        }
    }
}

impl std::error::Error for ParseError {}

pub struct Parser {}

impl Parser {
    pub fn parse(filename: &str) -> Result<HashMap<String, Program>, ParseError> {
        let file = File::open(filename).map_err(ParseError::OpenError)?;
        let mut config: Config = serde_yaml::from_reader(file).map_err(ParseError::YamlError)?;
        config.update();
        Ok(config.programs)
    }
}
