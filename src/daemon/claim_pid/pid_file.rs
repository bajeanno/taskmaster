use crate::daemon::error::ClaimError::{self};

use libc::sys::file::{LOCK_EX, LOCK_UN, flock};
use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Seek, Write},
    os::fd::AsRawFd,
};

#[derive(Debug)]
pub(super) struct PidFile {
    file: File,
}

impl PidFile {
    pub(super) fn open(pid_file_path: &str) -> Result<Self, ClaimError> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .truncate(false)
            .open(pid_file_path)
            .map_err(ClaimError::OpenFile)?;
        Ok(Self { file })
    }

    pub(super) unsafe fn read_pid(&mut self) -> Result<Option<u32>, ClaimError> {
        let mut buf = String::new();
        self.file
            .seek(io::SeekFrom::Start(0))
            .map_err(ClaimError::RewindFile)?;
        self.file
            .read_to_string(&mut buf)
            .map_err(ClaimError::ReadFile)?;
        Ok(match buf.is_empty() {
            true => None,
            false => Some(buf.parse::<u32>().map_err(ClaimError::Parse)?),
        })
    }

    pub(super) unsafe fn truncate(&mut self) {
        self.file.set_len(0).expect("File truncating should never return an error since the len is 0 and a PidFile only handles a file with write permissions");
    }

    pub(super) unsafe fn write_pid(&mut self) -> Result<(), ClaimError> {
        unsafe { self.truncate() };
        self.file
            .seek(io::SeekFrom::Start(0))
            .map_err(ClaimError::RewindFile)?;
        self.file
            .write_all(std::process::id().to_string().as_bytes())
            .map_err(ClaimError::WriteFile)?;
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn set_content_for_tests(&mut self, content: &str) {
        self.file
            .set_len(0)
            .expect("could not truncate the pid file");
        self.file
            .write_all(content.as_bytes())
            .expect("could not write into the pid file");
    }

    pub(super) fn fd(&self) -> i32 {
        self.file.as_raw_fd()
    }

    fn lock(&self) -> Result<(), ClaimError> {
        if unsafe { flock(self.fd(), LOCK_EX) == -1 } {
            return Err(ClaimError::Flock(io::Error::last_os_error()));
        }
        Ok(())
    }

    fn unlock(&self) -> Result<(), ClaimError> {
        if unsafe { flock(self.fd(), LOCK_UN) == -1 } {
            return Err(ClaimError::Flock(io::Error::last_os_error()));
        }
        Ok(())
    }

    pub(super) fn with_lock<F, T>(&mut self, call_back: F) -> Result<T, ClaimError>
    where
        F: FnOnce(&mut Self) -> Result<T, ClaimError>,
    {
        self.lock()?;
        let result = call_back(self);
        self.unlock()?;
        result
    }
}
