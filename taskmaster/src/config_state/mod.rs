use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

use crate::config::Config;
use std::io;
use std::io::Write;
use std::{fs::OpenOptions, io::Read, sync::Arc};
const CONF_FILE: &str = "/etc/taskmaster.conf";
const DEFAULT_TASKS_FILE: &str = "/etc/taskmaster.yaml";

#[cfg(test)]
mod tests;

#[allow(dead_code)]
#[derive(Default)]
pub enum ConfigState {
    Active(Arc<Config>),
    #[default]
    Uninitialized,
    LoadError {
        error: String,
    },
}

#[derive(Debug, Deserialize, Serialize)]
struct ConfFile {
    config_file_path: String,
}

#[derive(Debug, Error)]
pub enum ConfigFileError {
    #[error("Failed to parse taskmaster configuration file: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("Failed to open taskmaster configuration file: {0}")]
    Open(io::Error),
    #[error("Failed to read taskmaster configuration file: {0}")]
    Read(io::Error),
    #[error("Failed to write taskmaster configuration file: {0}")]
    Write(io::Error),
}

impl ConfigState {
    #[cfg(test)]
    pub fn from_content(content: String) -> Self {
        use std::io::Cursor;

        let config = Config::from_reader(Cursor::new(content)).expect("Parse error");
        Self::Active(Arc::new(config))
    }

    pub fn from_config(file: Option<&str>) -> Self {
        let mut config = Self::default();
        config.load_config(file).unwrap(); // TODO: write proper error handling
        config
    }

    pub fn load_config(&mut self, maybe_file: Option<&str>) -> Result<(), ConfigFileError> {
        self.load_config_with(CONF_FILE, maybe_file)
    }

    fn load_config_with(
        &mut self,
        conf_file_path: &str,
        maybe_file: Option<&str>,
    ) -> Result<(), ConfigFileError> {
        let file_path = match maybe_file {
            Some(file) => self.register_new_config_file(conf_file_path, Some(file))?,
            None => self.fetch_tasks_file_path(conf_file_path)?,
        };
        match Config::parse(file_path.as_str()) {
            Ok(config) => *self = Self::Active(Arc::new(config)),
            Err(err) => {
                eprintln!("Warning {err}"); //TODO: log error and/or broadcast to clients
                *self = Self::LoadError {
                    error: err.to_string(),
                };
            }
        };
        Ok(())
    }

    pub fn take(&mut self) -> Self {
        let mut tmp = Self::default();
        std::mem::swap(&mut tmp, self);
        tmp
    }

    fn fetch_tasks_file_path(&self, conf_file_path: &str) -> Result<String, ConfigFileError> {
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(conf_file_path)
            .map_err(ConfigFileError::Open)?;
        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(ConfigFileError::Read)?;
        Ok(toml::from_str::<ConfFile>(content.as_str())?.config_file_path)
    }

    fn register_new_config_file(
        &self,
        conf_file_path: &str,
        maybe_conf_file: Option<&str>,
    ) -> Result<String, ConfigFileError> {
        let conf_file = match maybe_conf_file {
            Some(conf_file_path) => ConfFile {
                config_file_path: conf_file_path.to_string(),
            },
            None => ConfFile {
                config_file_path: DEFAULT_TASKS_FILE.to_string(),
            },
        };
        let file_content = toml::to_string(&conf_file)
            .expect("error serializing ConfFile struct, see toml docs on Serialization failure");
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(conf_file_path)
            .map_err(ConfigFileError::Open)?;
        file.write_all(file_content.as_bytes())
            .map_err(ConfigFileError::Write)?;
        Ok(conf_file.config_file_path.to_string())
    }
}
