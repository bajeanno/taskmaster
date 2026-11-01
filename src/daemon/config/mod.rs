pub mod task;
pub use task::TaskConfig;

mod error;
pub use error::{CreatingDefaultConfigFileError, ParseError};

use serde::de::Error;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Config {
    pub tasks: HashMap<String, Arc<TaskConfig>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(test, derive(PartialEq))]
#[serde(deny_unknown_fields)]
struct TmpConfig {
    #[serde(with = "::serde_with::rust::maps_duplicate_key_is_error")]
    tasks: HashMap<String, TaskConfig>,
}

impl TmpConfig {
    fn tasks(self) -> Result<HashMap<String, TaskConfig>, serde_yaml::Error> {
        self.tasks
            .into_iter()
            .map(|(name, mut task)| {
                if name.contains(|c: char| c.is_ascii_digit()) {
                    return Err(serde_yaml::Error::custom(format!(
                        "task name '{name}' contains illegal characters (numerical value)",
                    )));
                }
                if name.starts_with("-") {
                    return Err(serde_yaml::Error::custom(format!(
                        "task name '{name}' may not start with '-'"
                    )));
                }

                *task.name_mut() = name.clone();
                Ok((name, task))
            })
            .collect()
    }

    fn template() -> Self {
        let mut config = Self {
            tasks: HashMap::new(),
        };
        config
            .tasks
            .insert("template_task".to_string(), TaskConfig::template());
        config
    }
}

impl From<Config> for TmpConfig {
    fn from(config_from: Config) -> Self {
        let mut config = Self {
            tasks: HashMap::new(),
        };
        for (name, task) in config_from.tasks {
            config.tasks.insert(name, task.deref().clone());
        }
        config
    }
}

impl Config {
    pub fn to_writer(&self, file: impl std::io::Write) -> Result<(), serde_yaml::Error> {
        serde_yaml::to_writer(file, &TmpConfig::from(self.clone()))?;
        Ok(())
    }

    pub fn from_reader(file: impl std::io::Read) -> Result<Config, serde_yaml::Error> {
        let tmp_config: TmpConfig = serde_yaml::from_reader(file)?;
        let config = Self {
            tasks: tmp_config
                .tasks()?
                .into_iter()
                .map(|(name, task)| (name, Arc::new(task)))
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
                                source: err,
                            }
                        })?,
                        &TmpConfig::template(),
                    )
                    .map_err(|err| {
                        CreatingDefaultConfigFileError::UnableToWrite {
                            file: file_name.to_string(),
                            source: err,
                        }
                    })?;
                    File::open(file_name)
                }
            }
        }
        .map_err(|err| ParseError::OpeningFile {
            file: file_name.to_string(),
            source: err,
        })?;

        Self::from_reader(file).map_err(|err| ParseError::InvalidConfig {
            file: file_name.to_string(),
            source: err,
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

        for (name, task) in config.tasks.iter_mut() {
            *task.name_mut() = name.clone();
        }
        assert_eq!(config, TmpConfig::template());
    }

    #[test]
    fn from_config_serialization_test() {
        let mut tasks = HashMap::new();
        let name = "task".to_string();
        tasks.insert(name.clone(), Arc::new(TaskConfig::template()));

        let original_config = Config { tasks: tasks };

        // Test From<Config> for TmpConfig
        let tmp_config = TmpConfig::from(original_config);

        // Serialize to YAML
        let serialized = serde_yaml::to_string(&tmp_config).unwrap();

        // Deserialize back to TmpConfig
        let deserialized_tmp: TmpConfig = serde_yaml::from_str(&serialized).unwrap();

        // Convert back to Config via from_reader logic and assert equality
        let final_config = Config {
            tasks: deserialized_tmp
                .tasks()
                .unwrap()
                .into_iter()
                .map(|(k, v)| (k, Arc::new(v)))
                .collect(),
        };

        // Re-wrap original for comparison (re-creating to avoid move issues)
        let mut expected_programs = HashMap::new();
        let mut expected_program = TaskConfig::template();
        *expected_program.name_mut() = name.clone();
        expected_programs.insert(name, Arc::new(expected_program));
        let expected_config = Config {
            tasks: expected_programs,
        };

        assert_eq!(final_config, expected_config);
    }
}
