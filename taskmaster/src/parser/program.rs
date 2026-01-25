use super::ParseError;
use derive_getters::Getters;
use libc::sys::types::Pid;
use serde::{Deserialize, Deserializer, de};
use signal::Signal;
use std::{collections::HashMap, fmt::Display, fs::File, str::FromStr};
use tokio::process::Command as TokioCommand;

#[derive(Debug, Deserialize, PartialEq)]
pub struct Config {
    pub programs: Vec<Program>,
}

#[derive(Debug)]
pub struct Command {
    pub command: TokioCommand,
    pub(super) string: String,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Default)]
pub enum AutoRestart {
    True,
    #[default]
    False,
    OnFailure,
}

#[allow(dead_code)] // TODO: remove this
#[derive(Debug, Getters, Deserialize, PartialEq)]
pub struct Program {
    #[serde(default)]
    pub(super) name: String, //defaults to yaml section name
    #[serde(default)]
    pub(super) pids: Vec<Pid>, //defaults to empty Vec
    #[serde(default = "default_umask", deserialize_with = "create_umask")]
    pub(super) umask: u32, //defaults to 0o666
    #[serde(deserialize_with = "create_command")]
    pub cmd: Command,
    #[serde(default = "default_num_procs")]
    pub(super) num_procs: u32, //defaults to 1
    #[serde(default = "default_work_dir")]
    pub(super) working_dir: String, //defaults to "/"
    #[serde(default)]
    pub(super) auto_start: bool, //defaults to False
    #[serde(default)]
    pub(super) auto_restart: AutoRestart, //defaults to AutoRestart::False
    #[serde(default = "default_exit_codes")]
    pub(super) exit_codes: Vec<u8>, //defaults to vec![0]
    #[serde(default)]
    pub(super) start_retries: u32, //defaults to 0
    #[serde(default)]
    pub(super) start_time: u32, //defaults to 0
    #[serde(default = "default_signal", deserialize_with = "deserialize_signal")]
    pub(super) stop_signal: Signal, //defaults to Signal::SIGINT
    #[serde(default)]
    pub(super) stop_time: u32, //defaults to 0
    #[serde(default = "default_output")]
    pub(super) stdout: String, //defaults to "/dev/null"
    #[serde(default = "default_output")]
    pub(super) stderr: String, //defaults to "/dev/null"
    #[serde(default, rename = "clearenv")]
    pub(super) clear_env: bool,
    #[serde(default)]
    pub(super) env: HashMap<String, String>, //defaults to empty HashMap
}

fn deserialize_signal<'de, D>(deserializer: D) -> Result<Signal, D::Error>
where
    D: Deserializer<'de>,
{
    let signal: Signal = Signal::from_str(String::deserialize(deserializer)?.as_str())
        .map_err(|_| de::Error::custom("Failed to convert signal from string"))?;
    Ok(signal)
}

fn create_umask<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    let umask_str = String::deserialize(deserializer)?;
    let umask = u32::from_str_radix(umask_str.as_str(), 8)
        .map_err(|_| serde::de::Error::custom("ParseIntError on umask parsing"))?;
    if umask > 0o777 {
        Err(serde::de::Error::custom(
            "umask is greater than 0o777 (max value accepted)",
        ))
    } else {
        Ok(umask)
    }
}

fn create_command<'de, D>(deserializer: D) -> Result<Command, D::Error>
where
    D: Deserializer<'de>,
{
    let cmd = String::deserialize(deserializer)?;
    let parts = shell_words::split(&cmd)
        .map_err(|_| serde::de::Error::custom("Failed to parse command"))?;

    let mut parts_iter = parts.into_iter();
    let program = parts_iter
        .next()
        .ok_or_else(|| serde::de::Error::custom("Empty command"))?;

    let mut command = Command {
        command: TokioCommand::new(program),
        string: cmd,
    };
    for arg in parts_iter {
        command.command.arg(arg);
    }

    Ok(command)
}

fn default_output() -> String {
    "/dev/null".to_string()
}

fn default_signal() -> Signal {
    Signal::SIGINT
}

fn default_num_procs() -> u32 {
    1
}

fn default_work_dir() -> String {
    String::from("/")
}

fn default_exit_codes() -> Vec<u8> {
    vec![0]
}

fn default_umask() -> u32 {
    0o666
}

impl PartialEq for Command {
    fn eq(&self, other: &Self) -> bool {
        self.string == other.string
    }
}

#[cfg(test)]
impl TryFrom<&str> for Program {
    type Error = ParseError;

    fn try_from(origin: &str) -> Result<Self, ParseError> {
        let mut result: Self = serde_yaml::from_str(origin)?;
        result.add_env();
        Ok(result)
    }
}

impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:<15}{:50}{: ^15?}{:>10o}",
            self.name, self.cmd.string, self.pids, self.umask,
        )
    }
}

impl Program {
    fn add_env(&mut self) {
        if self.clear_env {
            self.cmd.command.env_clear();
        }
        self.env.iter().for_each(|(key, val)| {
            self.cmd.command.env(key, val);
        });
    }
}

impl Config {
    #[cfg(test)]
    pub(super) fn push(&mut self, program: Program) {
        self.programs.push(program);
    }

    fn add_envs(&mut self) {
        self.programs
            .iter_mut()
            .for_each(|program| program.add_env());
    }

    pub(super) fn from_reader(file: impl std::io::Read) -> Result<Config, ParseError> {
        let map: HashMap<String, Program> =
            serde_yaml::from_reader(file).inspect_err(|err| eprintln!("{err}"))?;
        let mut config = Self {
            programs: map.into_values().collect(),
        };
        config.add_envs();
        Ok(config)
    }

    pub fn parse(file: &str) -> Result<Config, ParseError> {
        let file = File::open(file).map_err(ParseError::OpeningFile)?;
        let config = Self::from_reader(file)?;
        Ok(config)
    }
}
