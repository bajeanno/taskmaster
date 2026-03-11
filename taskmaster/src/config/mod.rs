pub mod program;
pub use program::Program;

mod error;
pub use error::ParseError;

use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Config {
    pub programs: Vec<Program>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TmpConfig {
    #[serde(with = "::serde_with::rust::maps_duplicate_key_is_error")]
    pub programs: HashMap<String, Program>,
}

impl Config {
    pub fn from_reader(file: impl std::io::Read) -> Result<Config, ParseError> {
        let tmp_config: TmpConfig = serde_yaml::from_reader(file)?;
        let config = Self {
            programs: tmp_config
                .programs
                .into_iter()
                .map(|(name, mut program)| {
                    *program.name_mut() = name;
                    program
                })
                .collect(),
        };
        Ok(config)
    }

    pub fn parse(filename: &str) -> Result<Config, ParseError> {
        let file = File::open(filename).map_err(|err| ParseError::OpeningFile {
            file: filename.to_string(),
            error: err,
        })?;
        Self::from_reader(file)
    }
}
