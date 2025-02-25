use libc::sys::types::Pid;
use serde::{Deserialize, Deserializer};
use std::{collections::HashMap, fmt::Display, fs::File};

use super::ParseError;

#[derive(Debug, Deserialize)]
pub struct ParsedConfig {
    pub programs: HashMap<String, ParsedProgram>,
}

pub struct Config {
    pub programs: Vec<Program>,
}

#[derive(Debug, PartialEq)]
enum AutoRestart {
    True,
    False,
    Unexpected,
}

impl<'de> Deserialize<'de> for AutoRestart {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "true" => Ok(AutoRestart::True),
            "false" => Ok(AutoRestart::False),
            "unexpected" => Ok(AutoRestart::Unexpected),
            _ => Err(serde::de::Error::custom("unexpected value")),
        }
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // TODO: remove this
pub struct EnvVar {
    key: String,
    value: String,
}

#[derive(Debug, Deserialize)]
pub struct ParsedProgram {
    pub name: Option<String>,
    cmd: String,
    umask: Option<String>,
    numprocs: Option<u32>,
    workingdir: Option<String>,
    autostart: Option<bool>,
    autorestart: Option<AutoRestart>,
    exitcodes: Option<Vec<u8>>,
    startretries: Option<u32>,
    starttime: Option<u32>,
    stopsignal: Option<String>,
    stoptime: Option<u32>,
    stdout: Option<String>,
    stderr: Option<String>,
    env: Option<HashMap<String, String>>,
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

impl ParsedProgram {
    fn check_signal(&self, name: &str) -> Result<String, ParseError> {
        match self
            .stopsignal
            .clone()
            .unwrap_or_else(|| String::from("INT"))
            .as_ref()
        {
            "HUP" => Ok(String::from("HUP")),
            "INT" => Ok(String::from("INT")),
            "QUIT" => Ok(String::from("QUIT")),
            "ILL" => Ok(String::from("ILL")),
            "TRAP" => Ok(String::from("TRAP")),
            "ABRT" => Ok(String::from("ABRT")),
            "EMT" => Ok(String::from("EMT")),
            "FPE" => Ok(String::from("FPE")),
            "KILL" => Ok(String::from("KILL")),
            "BUS" => Ok(String::from("BUS")),
            "SEGV" => Ok(String::from("SEGV")),
            "SYS" => Ok(String::from("SYS")),
            "PIPE" => Ok(String::from("PIPE")),
            "ALRM" => Ok(String::from("ALRM")),
            "TERM" => Ok(String::from("TERM")),
            "URG" => Ok(String::from("URG")),
            "STOP" => Ok(String::from("STOP")),
            "TSTP" => Ok(String::from("TSTP")),
            "CONT" => Ok(String::from("CONT")),
            "CHLD" => Ok(String::from("CHLD")),
            "TTIN" => Ok(String::from("TTIN")),
            "TTOU" => Ok(String::from("TTOU")),
            "IO" => Ok(String::from("IO")),
            "XCPU" => Ok(String::from("XCPU")),
            "XFSZ" => Ok(String::from("XFSZ")),
            "VTALRM" => Ok(String::from("VTALRM")),
            "PROF" => Ok(String::from("PROF")),
            "WINCH" => Ok(String::from("WINCH")),
            "INFO" => Ok(String::from("INFO")),
            "USR1" => Ok(String::from("USR1")),
            "USR2" => Ok(String::from("USR2")),
            sig => Err(ParseError::InvalidSignal(sig.to_string(), name.to_string())),
        }
    }
}

impl ParsedConfig {
    pub fn new(file: File) -> Result<Self, ParseError> {
        let new_config: Self = serde_yaml::from_reader(file)?;
        for (name, program) in &new_config.programs {
            program.check_signal(name)?;
        }
        Ok(new_config)
    }
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
        let file = File::open(filename).map_err(ParseError::FailedToOpenFile)?;
        let mut parsed_config = ParsedConfig::new(file)?;
        for (name, program) in &mut parsed_config.programs {
            program.name = Some(name.clone());
        }
        let config = Config::try_from(parsed_config)?;
        Ok(config.programs)
    }
}
