#[cfg(test)]
mod tests;

use crate::error::{
    Error,
    PidError::{self, OtherInstanceRunning},
};
use libc::sys::file::{LOCK_EX, LOCK_UN, flock};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::fd::AsRawFd,
};

#[cfg(not(test))]
const PID_FILE: &str = "/var/run/taskmaster.d/taskmaster.pid";
#[cfg(test)]
const PID_FILE: &str = "/tmp/taskmaster.pid";

#[derive(Debug)]
pub struct Claim();

impl Claim {
    pub fn new() -> Result<Self, Error> {
        let mut pid_file = PidFile::open()?;
        match pid_file.read_pid()? {
            Some(_) => Err(OtherInstanceRunning)?,
            None => {
                pid_file.write_pid()?;
                Ok(Claim())
            }
        }
    }

    pub fn release_claim(self) {
        // Drop implementation does the logic required to release the claim so we can leave this
        // empty
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        match PidFile::open() {
            Ok(mut file) => file.truncate(),
            Err(err) => {
                eprintln!("error: failed to open taskmaster's pid file at {PID_FILE}: {err}")
            }
        }
    }
}

#[derive(Debug)]
struct PidFile {
    file: File,
}

impl PidFile {
    fn open() -> Result<Self, Error> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .truncate(false)
            .open(PID_FILE)
            .map_err(PidError::OpenFile)?;
        unsafe { flock(file.as_raw_fd(), LOCK_EX) };
        Ok(Self { file })
    }

    fn read_pid(&mut self) -> Result<Option<u32>, Error> {
        let mut buf = String::new();
        self.file
            .read_to_string(&mut buf)
            .map_err(PidError::ReadFile)?;
        Ok(match buf.is_empty() {
            true => None,
            false => Some(buf.parse::<u32>().map_err(PidError::Parse)?),
        })
    }

    fn truncate(&mut self) {
        self.file.set_len(0).expect("File truncating should never return an error since the len is 0 and a PidFile only handles a file with write permissions");
    }

    fn write_pid(mut self) -> Result<(), Error> {
        self.file
            .write_all(std::process::id().to_string().as_bytes())
            .map_err(PidError::WriteFile)?;
        Ok(())
    }

    #[cfg(test)]
    fn set_content_for_tests(&mut self, content: &str) {
        self.file
            .set_len(0)
            .expect("could not truncate the pid file");
        self.file
            .write_all(content.as_bytes())
            .expect("could not write into the pid file");
    }
}

impl Drop for PidFile {
    fn drop(&mut self) {
        // Unlock pid file on drop to allow other processes to read and write inside
        unsafe { flock(self.file.as_raw_fd(), LOCK_UN) };
    }
}
