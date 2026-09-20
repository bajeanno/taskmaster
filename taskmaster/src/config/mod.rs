pub mod program;
pub use program::ProgramConfig;

mod default;
mod deserialize;
mod error;
mod serialize;
pub use error::{CreatingDefaultConfigFileError, ParseError};

use serde::de::Error;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;
use std::fs::File;
use std::sync::Arc;

#[derive(Debug, Deserialize, Serialize, Default, PartialEq)]
pub enum AutoRestart {
    #[serde(rename = "true")]
    True,
    #[default]
    #[serde(rename = "false")]
    False,
    #[serde(rename = "unexpected")]
    OnFailure,
}

#[derive(Debug, PartialEq)]
pub struct Command {
    pub exec: String,
    pub args: Vec<String>,
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}",
            self.exec,
            if !self.args.is_empty() {
                format!(" {:?}", self.args)
            } else {
                String::new()
            }
        )
    }
}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Config {
    pub programs: HashMap<String, Arc<ProgramConfig>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(test, derive(PartialEq))]
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

    fn template() -> Self {
        let mut config = Self {
            programs: HashMap::new(),
        };
        config
            .programs
            .insert("template_task".to_string(), ProgramConfig::template());
        config
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
        let file = {
            match File::open(file_name) {
                Ok(t) => Ok(t),
                Err(_) => {
                    serde_yaml::to_writer(
                        File::create(file_name).map_err(|err| {
                            CreatingDefaultConfigFileError::UnableToCreate {
                                file: file_name.to_string(),
                                error: err,
                            }
                        })?,
                        &TmpConfig::template(),
                    )
                    .map_err(|err| {
                        CreatingDefaultConfigFileError::UnableToWrite {
                            file: file_name.to_string(),
                            error: err,
                        }
                    })?;
                    File::open(file_name)
                }
            }
        }
        .map_err(|err| ParseError::OpeningFile {
            file: file_name.to_string(),
            error: err,
        })?;

        Self::from_reader(file).map_err(|err| ParseError::InvalidConfig {
            file: file_name.to_string(),
            error: err,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_test() {
        //TmpConfig is the same type as Config but with no Arc inside the HashMap
        let content = serde_yaml::to_string(&TmpConfig::template()).unwrap();
        let mut config: TmpConfig = serde_yaml::from_str(&content).unwrap();

        for (name, program) in config.programs.iter_mut() {
            *program.name_mut() = name.clone();
        }
        assert_eq!(config, TmpConfig::template());
    }
}
