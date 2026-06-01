pub mod program;
pub use program::ProgramConfig;

mod default;
mod deserialize;
mod error;
pub use error::ParseError;

use serde::Deserialize;
use serde::de::Error;
use std::collections::HashMap;
use std::fs::File;
use std::sync::Arc;

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Deserialize, Default)]
pub enum AutoRestart {
    #[serde(rename = "true")]
    True,
    #[default]
    #[serde(rename = "false")]
    False,
    #[serde(rename = "unexpected")]
    OnFailure,
}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Config {
    pub programs: HashMap<String, Arc<ProgramConfig>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TmpConfig {
    #[serde(with = "::serde_with::rust::maps_duplicate_key_is_error")]
    programs: HashMap<String, ProgramConfig>,
}

impl TmpConfig {
    fn programs(self) -> Result<HashMap<String, ProgramConfig>, serde_yaml::Error> {
        self.programs
            .into_iter()
            .map(|(name, mut program)| {
                if name.contains(|c: char| c.is_ascii_digit()) {
                    return Err(serde_yaml::Error::custom(format!(
                        "program name '{}' contains illegal characters (numerical value)",
                        name
                    )));
                }

                *program.name_mut() = name.clone();
                Ok((name, program))
            })
            .collect()
    }
}

impl Config {
    pub fn from_reader(file: impl std::io::Read) -> Result<Config, serde_yaml::Error> {
        let tmp_config: TmpConfig = serde_yaml::from_reader(file)?;
        let config = Self {
            programs: tmp_config
                .programs()?
                .into_iter()
                .map(|(name, program)| (name, Arc::new(program)))
                .collect(),
        };
        Ok(config)
    }

    pub fn parse(file_name: &str) -> Result<Config, ParseError> {
        let file = File::open(file_name).map_err(|err| ParseError::OpeningFile {
            file: file_name.to_string(),
            error: err,
        })?;

        Self::from_reader(file).map_err(|err| ParseError::InvalidConfig {
            file: file_name.to_string(),
            error: err,
        })
    }
}
