#[cfg(test)]
mod tests;

use super::default::{
    default_exit_codes, default_num_procs, default_signal, default_umask, default_work_dir,
};
use super::deserialize::{
    deserialize_num_procs, deserialize_signal, deserialize_stderr_file, deserialize_stdout_file,
    deserialize_umask,
};
use super::{AutoRestart, Command};
pub use crate::config::error::CommandError;
use crate::process_handler::OutputFile;
use derive_getters::Getters;
use libc::unistd::mode_t;
use serde::{Deserialize, Deserializer};
use signal::Signal;
use std::sync::Arc;
use std::{collections::HashMap, fmt::Display, str::FromStr};

#[allow(dead_code)] // TODO: remove this
#[derive(Debug, Getters, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProgramConfig {
    #[serde(skip)]
    name: String,

    #[serde(default = "default_umask", deserialize_with = "deserialize_umask")]
    umask: mode_t, //restart

    pub cmd: Command, //restart

    #[serde(
        rename = "numprocs",
        default = "default_num_procs",
        deserialize_with = "deserialize_num_procs"
    )]
    num_procs: u8,

    #[serde(rename = "workingdir", default = "default_work_dir")]
    working_dir: String, //restart

    #[serde(rename = "autostart", default)]
    auto_start: bool,

    #[serde(rename = "autorestart", default)]
    auto_restart: AutoRestart,

    #[serde(rename = "exitcodes", default = "default_exit_codes")]
    exit_codes: Vec<u8>,

    #[serde(rename = "startretries", default)]
    start_retries: u32,

    #[serde(rename = "starttime", default)]
    start_time: u32,

    #[serde(
        rename = "stopsignal",
        default = "default_signal",
        deserialize_with = "deserialize_signal"
    )]
    stop_signal: Signal,

    #[serde(rename = "stoptime", default)]
    stop_time: u32,

    #[serde(default, deserialize_with = "deserialize_stdout_file")]
    stdout: Arc<OutputFile>,

    #[serde(default, deserialize_with = "deserialize_stderr_file")]
    stderr: Arc<OutputFile>,

    #[serde(rename = "clearenv", default)]
    clear_env: bool,

    #[serde(default)]
    env: HashMap<String, String>, //restart
}

impl Display for ProgramConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:<15}{:50}", self.name, self.cmd,)
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {:?}", self.exec, self.args)
    }
}

impl ProgramConfig {
    pub(super) fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }
}

impl<'de> Deserialize<'de> for Command {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let cmd = String::deserialize(deserializer)?;
        Command::from_str(cmd.as_str())
            .map_err(|err| serde::de::Error::custom(format!("Command parsing error: {}", err)))
    }
}

impl FromStr for Command {
    type Err = CommandError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = shell_words::split(s).map_err(CommandError::SplitError)?;

        let mut parts_iter = parts.into_iter();
        let program = parts_iter.next().ok_or(CommandError::EmptyCommand)?;
        Ok(Command {
            exec: program,
            args: parts_iter.collect(),
        })
    }
}
