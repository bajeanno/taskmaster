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
    /// Name of the task
    ///
    /// *infered from the yaml section name*
    #[serde(skip)]
    name: String,

    /// The octal mask applied to permissions when interacting with the file system
    ///
    /// Needs restart of all the processes of the task on change
    ///
    /// Defaults to `0o022`
    #[serde(
        default = "default::umask",
        deserialize_with = "deserialize::umask",
        serialize_with = "serialize::umask"
    )]
    umask: mode_t,

    /// **Required field**
    ///
    /// The command the task will be running
    ///
    /// Needs restart of all the processes of the task on change
    pub cmd: Command,

    /// The number of processes the task will create
    ///
    /// Will shutdown or create new processes on change
    ///
    /// Defaults to `1`
    #[serde(
        default = "default::num_procs",
        deserialize_with = "deserialize::num_procs"
    )]
    num_procs: u8,

    /// The working directory the task will spawn in
    ///
    /// Needs restart of all the processes of the task on change
    ///
    /// Defaults to `/`
    #[serde(default = "default::work_dir")]
    working_dir: String,

    /// Whether the task starts automatically at daemon startup or not
    ///
    /// Defaults to `false`
    #[serde(default)]
    auto_start: bool,

    /// Whether the task starts automatically at reload or not
    ///
    /// Defaults to `false`
    #[serde(default)]
    auto_start_on_reload: bool,

    /// Whether the task restarts automatically, always, on failure, or never
    ///
    /// Defaults to `false`
    #[serde(default)]
    auto_restart: AutoRestart,

    /// The exit codes that means the task finished expectedly
    ///
    /// Defaults to `[0]`
    #[serde(default = "default::exit_codes")]
    exit_codes: Vec<u8>,

    /// The number of times taskmaster should retry starting the task if it fails unexpectedly before `start-time` seconds
    ///
    /// Defaults to `0`
    #[serde(default)]
    start_retries: u32,

    /// The time the task is expected to take before considering it has started
    ///
    /// Defaults to `0`
    #[serde(default)]
    start_time: u32,

    /// The signal that should be used to kill the process on first try
    ///
    /// Defaults to `SIGINT`
    #[serde(
        default = "default::signal",
        deserialize_with = "deserialize::signal",
        serialize_with = "serialize::signal"
    )]
    stop_signal: Signal,

    /// The time the task is expected to take while stopping before taskmaster tries to kill it with SIGKILL
    ///
    /// Defaults to `0`
    #[serde(default)]
    stop_time: u32,

    /// The path to stdout log file
    ///
    /// Needs restart of all the processes of the task on change
    ///
    /// Defaults to `/dev/null`
    #[serde(default, deserialize_with = "deserialize::stdout_file")]
    stdout: Arc<OutputFile>,

    /// The path to stderr log file
    ///
    /// Needs restart of all the processes of the task on change
    ///
    /// Defaults to `/dev/null`
    #[serde(default, deserialize_with = "deserialize::stderr_file")]
    stderr: Arc<OutputFile>,

    /// Wether the environnement should be cleared before starting or not
    ///
    /// Needs restart of all the processes of the task on change
    /// 
    /// Defaults to `false`
    #[serde(default)]
    clear_env: bool,

    /// The environnement variables the task will start with
    ///
    /// Needs restart of all the processes of the task on change
    ///
    /// Defaults to `[]`
    #[serde(default)]
    env: HashMap<String, String>,
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
            || self.clear_env() != other.clear_env()
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
