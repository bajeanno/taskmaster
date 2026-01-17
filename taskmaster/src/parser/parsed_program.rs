use crate::parser::{ParseError, program::AutoRestart};
use serde::{Deserialize, Deserializer};
use std::{collections::HashMap, ffi::c_int, fs::File};

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

#[cfg(test)]
impl TryFrom<&str> for ParsedProgram {
    type Error = ParseError;

    fn try_from(origin: &str) -> Result<Self, ParseError> {
        let result: Self = serde_yaml::from_str(origin)?;
        Ok(result)
    }
}

pub fn get_signal(signal: Option<&str>, name: &str) -> Result<c_int, ParseError> {
    match signal.unwrap_or("INT") {
        "HUP" => Ok(libc::signal::SIGHUP),
        "INT" => Ok(libc::signal::SIGINT),
        "QUIT" => Ok(libc::signal::SIGQUIT),
        "ILL" => Ok(libc::signal::SIGILL),
        "TRAP" => Ok(libc::signal::SIGTRAP),
        "ABRT" => Ok(libc::signal::SIGABRT),
        "EMT" => Ok(libc::signal::SIGEMT),
        "FPE" => Ok(libc::signal::SIGFPE),
        "KILL" => Ok(libc::signal::SIGKILL),
        "BUS" => Ok(libc::signal::SIGBUS),
        "SEGV" => Ok(libc::signal::SIGSEGV),
        "SYS" => Ok(libc::signal::SIGSYS),
        "PIPE" => Ok(libc::signal::SIGPIPE),
        "ALRM" => Ok(libc::signal::SIGALRM),
        "TERM" => Ok(libc::signal::SIGTERM),
        "URG" => Ok(libc::signal::SIGURG),
        "STOP" => Ok(libc::signal::SIGSTOP),
        "TSTP" => Ok(libc::signal::SIGTSTP),
        "CONT" => Ok(libc::signal::SIGCONT),
        "CHLD" => Ok(libc::signal::SIGCHLD),
        "TTIN" => Ok(libc::signal::SIGTTIN),
        "TTOU" => Ok(libc::signal::SIGTTOU),
        "IO" => Ok(libc::signal::SIGIO),
        "XCPU" => Ok(libc::signal::SIGXCPU),
        "XFSZ" => Ok(libc::signal::SIGXFSZ),
        "VTALRM" => Ok(libc::signal::SIGVTALRM),
        "PROF" => Ok(libc::signal::SIGPROF),
        "WINCH" => Ok(libc::signal::SIGWINCH),
        "INFO" => Ok(libc::signal::SIGINFO),
        "USR1" => Ok(libc::signal::SIGUSR1),
        "USR2" => Ok(libc::signal::SIGUSR2),
        sig => Err(ParseError::InvalidSignal(sig.to_string(), name.to_string())),
    }
}

impl ParsedProgram {
    fn set_name(&mut self, name: &str) {
        if self.name.is_none() {
            self.name = Some(name.to_string());
        }
    }
}

impl ParsedConfig {
    pub fn new(file: File) -> Result<Self, ParseError> {
        let mut new_config: Self = serde_yaml::from_reader(file)?;
        for (name, program) in &mut new_config.programs {
            get_signal(program.stopsignal.as_deref(), name)?;
            program.set_name(name);
        }
        Ok(new_config)
    }
}
