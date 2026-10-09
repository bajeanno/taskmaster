mod auto_restart;
mod command;
mod default;
mod deserialize;
mod serialize;

#[cfg(test)]
mod tests;

use crate::daemon::{config::error::CommandError, output_file::OutputFile};
pub use auto_restart::AutoRestart;
pub use command::Command;
use libc::unistd::mode_t;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use signal::Signal;
use std::{collections::HashMap, fmt::Display, str::FromStr, sync::Arc};

use derive_getters::Getters;

#[derive(Debug)]
pub enum ProgramDiff {
    NeedRestart,
    NumProcsChanged { before: usize, after: usize },
    Other,
}

#[allow(dead_code)] // TODO: remove this
#[derive(Debug, Getters, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ProgramConfig {
    #[serde(skip)]
    name: String,

    #[serde(
        default = "default::umask",
        deserialize_with = "deserialize::umask",
        serialize_with = "serialize::umask"
    )]
    umask: mode_t, //restart

    pub cmd: Command, //restart

    #[serde(
        default = "default::num_procs",
        deserialize_with = "deserialize::num_procs"
    )]
    num_procs: u8,

    #[serde(default = "default::work_dir")]
    working_dir: String, //restart

    #[serde(default)]
    auto_start: bool,

    #[serde(default)]
    auto_start_on_reload: bool,

    #[serde(default)]
    auto_restart: AutoRestart,

    #[serde(default = "default::exit_codes")]
    exit_codes: Vec<u8>,

    #[serde(default)]
    start_retries: u32,

    #[serde(default)]
    start_time: u32,

    #[serde(
        default = "default::signal",
        deserialize_with = "deserialize::signal",
        serialize_with = "serialize::signal"
    )]
    stop_signal: Signal,

    #[serde(default)]
    stop_time: u32,

    #[serde(default, deserialize_with = "deserialize::stdout_file")]
    stdout: Arc<OutputFile>,

    #[serde(default, deserialize_with = "deserialize::stderr_file")]
    stderr: Arc<OutputFile>,

    #[serde(default)]
    clear_env: bool,

    #[serde(default)]
    env: HashMap<String, String>, //restart
}

impl Display for ProgramConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:<15}{:50}", self.name, self.cmd,)
    }
}

impl ProgramConfig {
    pub(super) fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }

    pub fn diff(self: &Arc<Self>, other: &Arc<ProgramConfig>) -> ProgramDiff {
        if self.cmd != other.cmd
            || self.env() != other.env()
            || self.umask() != other.umask()
            || self.working_dir() != other.working_dir()
            || self.stdout() != other.stdout()
            || self.stderr() != other.stderr()
        {
            return ProgramDiff::NeedRestart;
        }

        if self.num_procs() != other.num_procs() {
            return ProgramDiff::NumProcsChanged {
                before: *self.num_procs() as usize,
                after: *other.num_procs() as usize,
            };
        }

        ProgramDiff::Other
    }

    pub(super) fn template() -> ProgramConfig {
        ProgramConfig {
            name: "template_task".to_string(),
            umask: 0o022,
            cmd: Command {
                exec: "echo".to_string(),
                args: vec!["Hello World!".to_string()],
            },
            num_procs: 1,
            working_dir: default::work_dir(),
            auto_start: true,
            auto_start_on_reload: false,
            auto_restart: AutoRestart::default(),
            exit_codes: vec![0],
            start_retries: 0,
            start_time: 0,
            stop_signal: Signal::SIGTERM,
            stop_time: 1,
            stdout: Arc::new(OutputFile::None),
            stderr: Arc::new(OutputFile::None),
            clear_env: false,
            env: HashMap::new(),
        }
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

impl Serialize for Command {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let str = String::from(&self.exec);
        let str = self
            .args
            .iter()
            .fold(str, |acc, arg| format!("{acc} \"{arg}\""));
        serializer.serialize_str(&str)
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
