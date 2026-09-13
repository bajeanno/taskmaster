use ron::ser::PrettyConfig;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

use crate::config::Config;
use crate::config_state::ConfigState::Active;
use std::fs::File;
use std::io;
use std::{fs::OpenOptions, sync::Arc};

const INIT_FILE: &str = "/etc/taskmaster.d/taskmaster.ron";
pub const DEFAULT_TASKS_FILE: &str = "/etc/taskmaster.d/taskmaster.yaml";

#[cfg(test)]
mod tests;

#[allow(dead_code)]
#[derive(Default)]
pub enum ConfigState {
    Active {
        config: Arc<Config>,
        config_file_path: String,
    },
    #[default]
    Uninitialized,
    LoadError {
        error: String,
        config_file_path: String,
    },
}

#[derive(Serialize)]
pub enum ReloadArgs {
    UseDefault,
    UseCurrent,
    NewDefault(String),
    TempConfig(String),
}

// InitFile is the struct that is serialized to ron (Rust Object Notation)
// and written to <INIT_FILE>
#[derive(Debug, Deserialize, Serialize, Clone)]
struct InitFile {
    default_config_file_path: String,
}

impl Default for InitFile {
    fn default() -> Self {
        Self {
            default_config_file_path: DEFAULT_TASKS_FILE.to_string(),
        }
    }
}

impl InitFile {
    fn new() -> Self {
        Self::default()
    }

    fn flush(self) -> Result<Self, InitFileError> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(INIT_FILE)
            .map_err(InitFileError::Open)?;

        ron::Options::default()
            .to_io_writer_pretty(file, &self, PrettyConfig::new().struct_names(true))
            .expect("error serializing InitFile struct, see toml docs on Serialization failure");
        Ok(self)
    }

    fn fetch() -> Result<Self, InitFileError> {
        let file = match OpenOptions::new()
            .read(true)
            .open(INIT_FILE)
            .map_err(InitFileError::Open)
        {
            Ok(file) => file,
            Err(_) => {
                Self::flush(Self::new())?;
                return Ok(Self::new());
            }
        };
        ron::de::from_reader::<File, InitFile>(file).map_err(InitFileError::Parse)
    }
}

impl From<String> for InitFile {
    fn from(value: String) -> Self {
        Self {
            default_config_file_path: value,
        }
    }
}

#[derive(Debug, Error)]
pub enum InitFileError {
    #[error("Failed to parse taskmaster configuration file: {0}")]
    Parse(#[from] ron::de::SpannedError),
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
        Self::Active {
            config: Arc::new(config),
            config_file_path: DEFAULT_TASKS_FILE.into(),
        }
    }

    pub fn from_default_config_file() -> Result<Self, InitFileError> {
        Self::from_config_file(ReloadArgs::UseDefault)
    }

    pub fn from_config_file(reload_command: ReloadArgs) -> Result<Self, InitFileError> {
        Self::default().load_config(reload_command)
    }

    pub fn load_config(&self, reload_command: ReloadArgs) -> Result<Self, InitFileError> {
        let config_file_path = self.get_file_path_to_use(reload_command)?;
        match Config::parse(&config_file_path) {
            Ok(config) => Ok(Self::Active {
                config: Arc::new(config),
                config_file_path,
            }),
            Err(err) => {
                Ok(Self::LoadError {
                    error: err.to_string(),
                    config_file_path,
                })
            }
        }
    }

    fn get_file_path_to_use(&self, reload_command: ReloadArgs) -> Result<String, InitFileError> {
        Ok(match reload_command {
            ReloadArgs::UseDefault => InitFile::fetch()?.default_config_file_path,
            ReloadArgs::UseCurrent => match self {
                Active {
                    config: _,
                    config_file_path,
                } => config_file_path.clone(),
                ConfigState::Uninitialized => InitFile::fetch()?.default_config_file_path,
                ConfigState::LoadError {
                    error: _,
                    config_file_path,
                } => config_file_path.clone(),
            },
            ReloadArgs::NewDefault(path) => {
                InitFile::from(path.clone()).flush()?;
                path
            }
            ReloadArgs::TempConfig(path) => path,
        })
    }

    pub fn take(&mut self) -> Self {
        let mut tmp = Self::default();
        std::mem::swap(&mut tmp, self);
        tmp
    }
}
