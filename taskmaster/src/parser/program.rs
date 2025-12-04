use super::ParseError;
use super::parsed_program::{EnvVar, ParsedConfig, ParsedProgram};
use derive_getters::Getters;
use libc::sys::types::Pid;
use std::{fmt::Display, fs::File};
use tokio::process::Command;

pub struct Config {
    pub programs: Vec<Program>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AutoRestart {
    True,
    False,
    Unexpected,
}

#[derive(Debug, Getters)]
#[allow(dead_code)] // TODO: remove this
pub struct Program {
    name: String,
    pids: Vec<Pid>,
    umask: u32,
    pub cmd: Command,
    cmd_str: String,
    num_procs: u32,
    working_dir: String,
    auto_start: bool,
    auto_restart: AutoRestart,
    exit_codes: Vec<u8>,
    start_retries: u32,
    start_time: u32,
    stop_signal: String, // check for valid signal
    stop_time: u32,
    stdout: String,
    stderr: String,
}

impl TryFrom<ParsedConfig> for Config {
    type Error = ParseError;

    fn try_from(origin: ParsedConfig) -> Result<Self, ParseError> {
        let mut programs = Vec::new();
        for (_, value) in origin.programs {
            programs.push(Program::try_from(value)?);
        }
        Ok(Config { programs })
    }
}

pub fn create_command(cmd: String, name: &String) -> Result<Command, ParseError> {
    let mut command: Command;
    let mut iter = shell_words::split(&cmd)?.into_iter();
    if let Some(program) = iter.next() {
        command = Command::new(program);
        iter.for_each(|arg| {
            command.arg(arg);
        });
        Ok(command)
    } else {
        Err(ParseError::EmptyCommand(name.to_string()))
    }
}

impl TryFrom<ParsedProgram> for Program {
    type Error = ParseError;

    fn try_from(origin: ParsedProgram) -> Result<Self, ParseError> {
        let umask_str = origin.umask.clone().unwrap_or_else(|| String::from("000"));
        let umask = u32::from_str_radix(&umask_str, 8).map_err(|_| {
            ParseError::InvalidUmask(
                "Invalid umask".to_string(),
                origin.name.clone().unwrap_or_else(|| String::from("")),
            )
        })?;
        if umask >= 0o777 {
            return Err(ParseError::InvalidUmask(
                "Invalid umask".to_string(),
                origin.name.unwrap_or_else(|| String::from("")),
            ));
        }
        let name = origin.name.unwrap_or_else(|| String::from(""));
        let mut cmd = create_command(origin.cmd.clone(), &name)?;
        let env = match origin.env {
            Some(x) => x
                .into_iter()
                .map(|(key, value)| EnvVar { key, value })
                .collect::<Vec<EnvVar>>(),
            None => Vec::new(),
        };
        cmd.env_clear();
        env.iter().for_each(|var| {
            cmd.env(&var.key, &var.value);
        });
        let result = Self {
            name,
            pids: Vec::new(),
            cmd,
            cmd_str: origin.cmd.clone(),
            umask,
            num_procs: origin.numprocs.unwrap_or(1),
            working_dir: origin.workingdir.unwrap_or_else(|| String::from("/")),
            auto_start: origin.autostart.unwrap_or(true),
            auto_restart: origin.autorestart.unwrap_or(AutoRestart::False),
            exit_codes: origin.exitcodes.unwrap_or_else(|| Vec::from([0])),
            start_retries: origin.startretries.unwrap_or(0),
            start_time: origin.starttime.unwrap_or(0),
            stop_signal: origin.stopsignal.unwrap_or_else(|| String::from("INT")), // check for valid signal
            stop_time: origin.stoptime.unwrap_or(5),
            stdout: origin.stdout.unwrap_or_else(|| String::from("/dev/null")),
            stderr: origin.stderr.unwrap_or_else(|| String::from("/dev/null")),
        };
        Ok(result)
    }
}

#[cfg(test)]
impl TryFrom<&str> for Program {
    type Error = ParseError;

    fn try_from(origin: &str) -> Result<Self, ParseError> {
        Self::try_from(ParsedProgram::try_from(origin)?)
    }
}

impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:<15}{:50}{: ^15?}{:>10o}",
            self.name.clone(),
            self.cmd_str.clone(),
            self.pids,
            self.umask,
        )
    }
}

impl Config {
    pub fn parse(filename: &str) -> Result<Vec<Program>, ParseError> {
        let file = File::open(filename).map_err(ParseError::OpeningFile)?;
        let mut parsed_config = ParsedConfig::new(file)?;
        for (name, program) in &mut parsed_config.programs {
            program.name = Some(name.clone());
        }
        let config = Config::try_from(parsed_config)?;
        Ok(config.programs)
    }
}
