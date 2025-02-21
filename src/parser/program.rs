use libc::sys::types::Pid;
use serde::Deserialize;
use std::{collections::HashMap, fmt::Display, fs::File};

use super::ParseError;

#[derive(Debug, Deserialize)]
pub struct ParsedConfig {
    pub programs: HashMap<String, ParsedProgram>,
}

pub struct Config {
    pub programs: Vec<Program>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // TODO: remove this
pub struct EnvVar {
    key: String,
    value: String,
}

#[derive(Debug, Deserialize)]
pub struct ParsedProgram {
    cmd: String,
    numprocs: Option<u32>,
    workingdir: Option<String>,
    autostart: Option<bool>,
    exitcodes: Option<Vec<u8>>, // check for valid codes (%256)
    startretries: Option<u32>,
    starttime: Option<u32>,
    stopsignal: Option<String>, // check for valid signal
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
    cmd: String,
    numprocs: u32,
    workingdir: String,
    autostart: bool,
    exitcodes: Vec<u8>, // check for valid codes (%256)
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

impl From<ParsedConfig> for Config {
    fn from(origin: ParsedConfig) -> Self {
        Self {
            programs: origin
                .programs
                .into_iter()
                .fold(vec![], |mut acc, (key, value)| {
                    acc.push(Program::from(key, value));
                    acc
                }),
        }
    }
}

impl Program {
    fn from(name: String, origin: ParsedProgram) -> Self {
        Self {
            name,
            pids: Vec::new(),
            cmd: origin.cmd,
            numprocs: origin.numprocs.unwrap_or(1),
            workingdir: origin.workingdir.unwrap_or_else(|| String::from("/")),
            autostart: origin.autostart.unwrap_or(true),
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
        }
    }
}

impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:<15}{:50}{:?}",
            self.name.clone(),
            self.cmd.clone(),
            self.pids,
        )
    }
}
