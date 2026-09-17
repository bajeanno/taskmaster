use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::fd::AsRawFd,
};

use crate::error::{Error, PidError};
use libc::sys::file::{LOCK_EX, LOCK_UN, flock};

const PID_FILE: &str = "/var/run/taskmaster.d/taskmaster.pid";

#[derive(Debug)]
pub struct PidFile {
    file: File,
}

impl PidFile {
    pub fn open() -> Result<Self, Error> {
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

    pub fn read_pid(&mut self) -> Result<Option<u32>, Error> {
        let mut buf = String::new();
        self.file
            .read_to_string(&mut buf)
            .map_err(PidError::ReadFile)?;
        Ok(match buf.is_empty() {
            true => None,
            false => Some(buf.parse::<u32>().map_err(PidError::Parse)?),
        })
    }

    pub fn truncate(&mut self) {
        self.file.set_len(0).expect("File truncating should never return an error since the len is 0 and a PidFile only handles a file with write permissions");
    }

    pub fn write_pid(&mut self) -> Result<(), Error> {
        self.file
            .write_all(std::process::id().to_string().as_bytes())
            .map_err(PidError::WriteFile)?;
        Ok(())
    }
}

impl Drop for PidFile {
    fn drop(&mut self) {
        // Unlock pid file on drop to allow other processes to read and write inside
        unsafe { flock(self.file.as_raw_fd(), LOCK_UN) };
    }
}

#[cfg(test)]
impl PidFile {
    pub(crate) fn set_content_for_tests(&mut self, content: &str) {
        self.file
            .set_len(0)
            .expect("could not truncate the pid file");
        self.file
            .write_all(content.as_bytes())
            .expect("could not write into the pid file");
    }
}
