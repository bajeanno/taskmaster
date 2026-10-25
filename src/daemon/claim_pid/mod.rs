#[cfg(test)]
mod tests;

mod pid_file;

use signal::Signal;

use crate::{
    daemon::{
        claim_pid::{CheckInstanceAlive::ParsePgrep, pid_file::PidFile},
        error::ClaimError::{self, OtherInstanceRunning},
    },
    rpc::RPC_UNIX_SOCKET_PATH,
    start_mode::StartMode,
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
    file: PidFile,
}

impl Claim {
    pub fn is_claimed() -> Result<bool, ClaimError> {
        Ok(Self::get_running_pid()?.is_some())
    }

    pub fn new(start_mode: StartMode) -> Result<Self, ClaimError> {
        Self::new_at(PID_FILE, start_mode)
    }

    fn new_at(pid_file_path: &str, start_mode: StartMode) -> Result<Self, ClaimError> {
        let mut pid_file = PidFile::open(pid_file_path)?;

        pid_file.with_lock(|file| {
            match unsafe { file.read_pid()? } {
                Some(pid) => match start_mode {
                    StartMode::Force { .. } => {
                        Self::kill_running_instance(pid as c_int)?;
                        let _ = std::fs::remove_file(RPC_UNIX_SOCKET_PATH);
                    }
                    StartMode::Acknowledge { .. } | StartMode::Classic => {
                        Err(OtherInstanceRunning)?
                    }
                },
                None => {}
            }
            unsafe { file.write_pid()? };
            Ok(())
        })?;

        Ok(Self { file: pid_file })
    }

    pub fn release_claim(self) {
        // Drop implementation does the logic required to release the claim so we can leave this
        // empty
    }

    fn get_running_pid() -> Result<Option<u32>, ClaimError> {
        let mut pid_file = PidFile::open(PID_FILE)?;
        pid_file.with_lock(|file| unsafe { file.read_pid() })
    }

    fn kill_running_instance(pid: c_int) -> Result<(), CheckInstanceAlive> {
        if cfg!(debug_assertions) {
            // Not checking process name in debug mode as integration tests will be named "test-.."
            unsafe { kill(pid, Signal::SIGKILL as c_int) };
            return Ok(());
        }

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
                unsafe { kill(pid, Signal::SIGKILL as c_int) };
                return Ok(());
            }
        }
        Ok(())
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        match self.file.with_lock(|file| {
            unsafe { file.truncate() };
            Ok(())
        }) {
            Ok(_) => {}
            Err(err) => logging::log_with_msg(
                "Error while clearing pid file",
                &err,
                logging::LogLevel::Error,
            ),
        }
    }
}
