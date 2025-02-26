use super::parsed_program::{AutoRestart, EnvVar, ParsedConfig, ParsedProgram};
use libc::sys::types::Pid;
use std::{fmt::Display, fs::File};

use super::ParseError;

pub struct Config {
    pub programs: Vec<Program>,
}

#[derive(Debug)]
#[allow(dead_code)] // TODO: remove this
pub struct Program {
    name: String,
    pids: Vec<Pid>,
    umask: u32,
    cmd: String,
    numprocs: u32,
    workingdir: String,
    autostart: bool,
    autorestart: AutoRestart,
    exitcodes: Vec<u8>,
    startretries: u32,
    starttime: u32,
    stopsignal: String, // check for valid signal
    stoptime: u32,
    stdout: String,
    stderr: String,
    env: Vec<EnvVar>,
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
        if umask >= 512 {
            return Err(ParseError::InvalidUmask(
                "Invalid umask".to_string(),
                origin.name.unwrap_or_else(|| String::from("")),
            ));
        }
        let result = Self {
            name: origin.name.unwrap_or_else(|| String::from("")),
            pids: Vec::new(),
            cmd: origin.cmd,
            umask,
            numprocs: origin.numprocs.unwrap_or(1),
            workingdir: origin.workingdir.unwrap_or_else(|| String::from("/")),
            autostart: origin.autostart.unwrap_or(true),
            autorestart: origin.autorestart.unwrap_or(AutoRestart::True),
            exitcodes: origin.exitcodes.unwrap_or_else(|| Vec::from([0])),
            startretries: origin.startretries.unwrap_or(0),
            starttime: origin.starttime.unwrap_or(5),
            stopsignal: origin.stopsignal.unwrap_or_else(|| String::from("INT")), // check for valid signal
            stoptime: origin.stoptime.unwrap_or(5),
            stdout: origin.stdout.unwrap_or_else(|| String::from("/dev/null")),
            stderr: origin.stderr.unwrap_or_else(|| String::from("/dev/null")),
            env: match origin.env {
                Some(x) => x
                    .into_iter()
                    .map(|(key, value)| EnvVar { key, value })
                    .collect::<Vec<EnvVar>>(),
                None => Vec::new(),
            },
        };
        Ok(result)
    }
}

impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:<15}{:50}{: ^15?}{:>10o}",
            self.name.clone(),
            self.cmd.clone(),
            self.pids,
            self.umask,
        )
    }
}

impl Config {
    pub fn parse(filename: &str) -> Result<Vec<Program>, ParseError> {
        let file = File::open(filename).map_err(ParseError::OpenError)?;
        let mut parsed_config = ParsedConfig::new(file)?;
        for (name, program) in &mut parsed_config.programs {
            program.name = Some(name.clone());
        }
        let config = Config::try_from(parsed_config)?;
        Ok(config.programs)
    }
}
