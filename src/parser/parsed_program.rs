use serde::{Deserialize, Deserializer};
use std::{collections::HashMap, fs::File};
use crate::parser::ParseError;

#[derive(Debug, PartialEq)]
pub enum AutoRestart {
    True,
    False,
    Unexpected,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // TODO: remove this
pub struct EnvVar {
    pub key: String,
    pub value: String,
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
pub struct ParsedConfig {
    pub programs: HashMap<String, ParsedProgram>,
}

#[derive(Debug, Deserialize)]
pub struct ParsedProgram {
    pub name: Option<String>,
    pub cmd: String,
    pub umask: Option<String>,
    pub numprocs: Option<u32>,
    pub workingdir: Option<String>,
    pub autostart: Option<bool>,
    pub autorestart: Option<AutoRestart>,
    pub exitcodes: Option<Vec<u8>>,
    pub startretries: Option<u32>,
    pub starttime: Option<u32>,
    pub stopsignal: Option<String>,
    pub stoptime: Option<u32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub env: Option<HashMap<String, String>>,
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
