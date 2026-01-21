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

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub enum AutoRestart {
    True,
    False,
    OnFailure,
}

//TODO: check deafult values
#[allow(dead_code)] // TODO: remove this
#[derive(Debug, Getters, Deserialize)]
pub struct Program {
    #[serde(default)]
    name: String,
    #[serde(default)]
    pids: Vec<Pid>,
    #[serde(deserialize_with = "create_umask")]
    umask: u32,
    #[serde(deserialize_with = "create_command")] //TODO: add env to command
    pub cmd: Command,
    #[serde(default)]
    num_procs: u32,
    #[serde(default)]
    working_dir: String,
    #[serde(default)]
    auto_start: bool,
    #[serde(default)]
    auto_restart: AutoRestart,
    #[serde(default)]
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
    env: HashMap<String, String>,
}

impl Default for AutoRestart {
    fn default() -> Self {
        Self::False
    }
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
        self.env.iter().for_each(|(key, val)| {
            self.cmd.command.env(key, val);
            println!("putting {key}: {val} in env")
        });
    }
}

impl Config {
    fn add_envs(&mut self) {
        self.programs
            .iter_mut()
            .for_each(|program| program.add_env());
    }

    pub fn parse(file: &str) -> Result<Vec<Program>, ParseError> {
        let file = File::open(file).map_err(ParseError::OpeningFile)?;
        let mut config: Self =
            serde_yaml::from_reader(file).inspect_err(|err| eprintln!("{err}"))?;
        config.add_envs();
        Ok(config.programs)
    }
}
