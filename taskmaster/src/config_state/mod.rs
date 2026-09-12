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
        Self::new()
    }
}

struct FmtWriter<W: io::Write>(W);
impl<W: io::Write> std::fmt::Write for FmtWriter<W> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.write_all(s.as_bytes()).map_err(|_| std::fmt::Error)
    }
}

impl InitFile {
    fn new() -> Self {
        Self {
            default_config_file_path: DEFAULT_TASKS_FILE.into(),
        }
    }

    fn flush(self) -> Result<Self, InitFileError> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(INIT_FILE)
            .map_err(InitFileError::Open)?;

        ron::ser::to_writer_pretty(FmtWriter(file), &self, PrettyConfig::new().struct_names(true)).expect(
                "error serializing InitFile struct, see toml docs on Serialization failure",
            );
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
        Self { default_config_file_path: value }
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

    pub fn from_default_config_file() -> Self {
        Self::from_config_file(ReloadArgs::UseDefault)
    }

    pub fn from_config_file(reload_command: ReloadArgs) -> Self {
        let config = Self::default();
        config.load_config(reload_command).unwrap(); // TODO: write proper error handling
        config
    }

    pub fn load_config(&self, reload_command: ReloadArgs) -> Result<Self, InitFileError> {
        let config_file_path = self.get_file_path_to_use(reload_command)?;
        match Config::parse(&config_file_path) {
            Ok(config) => Ok(Self::Active {
                config: Arc::new(config),
                config_file_path: config_file_path,
            }),
            Err(err) => {
                eprintln!("{err}"); //TODO: log error and/or broadcast to clients
                Ok(Self::LoadError {
                    error: err.to_string(),
                })
            }
        }
    }

    fn get_file_path_to_use(&self, reload_command: ReloadArgs) -> Result<String, InitFileError> {
        Ok(match reload_command {
            ReloadArgs::UseDefault => InitFile::fetch()?.default_config_file_path,
            ReloadArgs::UseCurrent => {
                if let Active {
                    config: _,
                    config_file_path: current_config_file,
                } = self
                {
                    current_config_file.to_string()
                } else {
                    InitFile::fetch()?.default_config_file_path
                }
            }
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
