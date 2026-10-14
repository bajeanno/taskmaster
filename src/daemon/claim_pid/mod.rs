#[cfg(test)]
mod tests;

mod pid_file;
mod start_mode;

use signal::Signal;
use start_mode::StartMode;

use crate::daemon::{
    claim_pid::{CheckInstanceAlive::ParsePgrep, pid_file::PidFile},
    error::ClaimError::{self, OtherInstanceRunning},
};

use libc::signal::kill;
use std::{
    io::{self},
    os::raw::c_int,
    process::{Command, Stdio},
    str::FromStr,
};

#[cfg(not(test))]
const PID_FILE: &str = "/var/run/taskmaster.d/taskmaster.pid";
#[cfg(test)]
const PID_FILE: &str = "target/tests/pid/taskmaster.pid";

#[derive(Debug, thiserror::Error)]
pub enum CheckInstanceAlive {
    #[error("Failed to parse output of pgrep")]
    ReadPids(#[source] io::Error),
    #[error("Failed to parse pid from pgrep output")]
    SpawnPgrep(#[source] io::Error),
    #[error("Failed to parse pgrep output")]
    ParsePgrep(#[source] <i32 as FromStr>::Err),
}

#[derive(Debug)]
pub struct Claim {
    pid_file_path: String,
}

impl Claim {
    pub fn is_claimed() -> Result<bool, ClaimError> {
        Ok(Self::get_running_pid()?.is_some())
    }

    pub fn new() -> Result<Self, ClaimError> {
        let start_mode = StartMode::from_args();
        let pid_file_path = PID_FILE.to_string();
        match start_mode {
            StartMode::Force => {
                if let Some(pid) = Self::get_running_pid()? {
                    Self::kill_running_instance(pid as i32).map_err(ClaimError::Kill)?;
                    Self::clear_pid_file_at(&pid_file_path);
                }
            }
            StartMode::Classic => {}
        }
        Self::new_at(pid_file_path)
    }

    fn new_at(pid_file_path: String) -> Result<Self, ClaimError> {
        let mut pid_file = PidFile::open(&pid_file_path)?;
        match pid_file.read_pid()? {
            Some(_) => Err(OtherInstanceRunning)?,
            None => {
                pid_file.write_pid()?;
                Ok(Self { pid_file_path })
            }
        }
    }

    pub fn release_claim(self) {
        // Drop implementation does the logic required to release the claim so we can leave this
        // empty
    }

    fn get_running_pid() -> Result<Option<u32>, ClaimError> {
        let mut pid_file = PidFile::open(PID_FILE)?;
        pid_file.read_pid()
    }

    fn clear_pid_file_at(path: &str) {
        match PidFile::open(path) {
            Ok(mut file) => file.truncate(),
            Err(err) => eprintln!(
                "error: failed to open taskmaster's pid file at {}: {err}",
                path
            ),
        }
    }

    fn kill_running_instance(pid: i32) -> Result<(), CheckInstanceAlive> {
        let output = Command::new("pgrep")
            .arg("taskmaster")
            .stdout(Stdio::piped())
            .spawn()
            .map_err(CheckInstanceAlive::SpawnPgrep)?
            .wait_with_output()
            .map_err(CheckInstanceAlive::ReadPids)?;

        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let pgrep_pid = line.parse::<i32>().map_err(ParsePgrep)?;

            if pid == pgrep_pid {
                unsafe { kill(pid as c_int, Signal::SIGKILL as c_int) };
                return Ok(());
            }
        }
        Ok(())
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        Self::clear_pid_file_at(&self.pid_file_path);
    }
}
