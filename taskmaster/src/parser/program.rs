use super::ParseError;
use derive_getters::Getters;
use libc::sys::types::Pid;
use serde::{Deserialize, Deserializer, de};
use signal::Signal;
use std::{collections::HashMap, fmt::Display, fs::File, str::FromStr};
use tokio::process::Command as TokioCommand;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub programs: Vec<Program>,
}

#[derive(Debug)]
pub struct Command {
    pub command: TokioCommand,
    string: String,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Default)]
pub enum AutoRestart {
    True,
    #[default]
    False,
    OnFailure,
}

#[allow(dead_code)] // TODO: remove this
#[derive(Debug, Getters, Deserialize)]
pub struct Program {
    #[serde(default)]
    name: String,
    #[serde(default)]
    pids: Vec<Pid>,
    #[serde(default = "default_umask", deserialize_with = "deserialize_umask")]
    umask: u32,
    #[serde(deserialize_with = "deserialize_command")]
    pub cmd: Command,
    #[serde(default = "default_num_procs")]
    num_procs: u32,
    #[serde(default = "default_work_dir")]
    working_dir: String,
    #[serde(default)]
    auto_start: bool,
    #[serde(default)]
    auto_restart: AutoRestart,
    #[serde(default = "default_exit_codes")]
    exit_codes: Vec<u8>,
    #[serde(default)]
    start_retries: u32,
    #[serde(default)]
    start_time: u32,
    #[serde(default = "default_signal", deserialize_with = "deserialize_signal")]
    stop_signal: Signal,
    #[serde(default)]
    stop_time: u32,
    #[serde(default = "default_output")]
    stdout: String,
    #[serde(default = "default_output")]
    stderr: String,
    #[serde(default)]
    clear_env: bool,
    #[serde(default)]
    env: HashMap<String, String>,
}

fn deserialize_signal<'de, D>(deserializer: D) -> Result<Signal, D::Error>
where
    D: Deserializer<'de>,
{
    let signal: Signal = Signal::from_str(
        String::deserialize(deserializer)
            .map_err(|err| serde::de::Error::custom(format!("Failed to parse signal: {err}")))?
            .as_str(),
    )
    .map_err(|err| de::Error::custom(format!("Failed to convert signal from string: {err}")))?;
    Ok(signal)
}

fn deserialize_umask<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    let umask_str = String::deserialize(deserializer)
        .map_err(|err| serde::de::Error::custom(format!("Failed to parse umask: {err}")))?;
    let umask = u32::from_str_radix(umask_str.as_str(), 8).map_err(|err| {
        serde::de::Error::custom(format!("ParseIntError on umask parsing: {err}"))
    })?;
    if umask > 0o777 {
        Err(serde::de::Error::custom(
            "umask is greater than 0o777 (max value accepted)",
        ))
    } else {
        Ok(umask)
    }
}

fn deserialize_command<'de, D>(deserializer: D) -> Result<Command, D::Error>
where
    D: Deserializer<'de>,
{
    let cmd = String::deserialize(deserializer)
        .map_err(|err| serde::de::Error::custom(format!("Failed to parse command: {err}")))?;
    let parts = shell_words::split(&cmd)
        .map_err(|err| serde::de::Error::custom(format!("Failed to parse command: {err}")))?;

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
    fn add_envs(&mut self) {
        self.programs
            .iter_mut()
            .for_each(|program| program.add_env());
    }

    pub fn from_reader(file: impl std::io::Read) -> Result<Config, ParseError> {
        let map: HashMap<String, Program> = serde_yaml::from_reader(file)?;
        let mut config = Self {
            programs: map
                .into_iter()
                .map(|(name, mut program)| {
                    if program.name.is_empty() {
                        program.name = name;
                    }
                    program
                })
                .collect(),
        };
        config.add_envs();
        Ok(config)
    }

    pub fn parse(file: &str) -> Result<Config, ParseError> {
        let file = File::open(file).map_err(ParseError::OpeningFile)?;
        Self::from_reader(file)
    }
}
