use crate::daemon::error::ClaimError::{self};

use std::{
    fs::OpenOptions,
    io::{self, Read, Seek, Write},
};

use file_lock::{FileLock, FileLockGuard};

#[derive(Debug)]
pub(super) struct PidFileLock {
    file: FileLock,
}

pub(super) struct PidFileLockGuard<'a> {
    file: FileLockGuard<'a>,
}

impl PidFileLock {
    pub(super) fn open(pid_file_path: &str) -> Result<Self, ClaimError> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .truncate(false)
            .open(pid_file_path)
            .map_err(ClaimError::OpenFile)?;
        Ok(Self {
            file: FileLock::new(file),
        })
    }

    pub fn lock(&self) -> Result<PidFileLockGuard<'_>, io::Error> {
        Ok(PidFileLockGuard {
            file: self.file.lock()?,
        })
    }

    #[cfg(test)]
    pub(super) fn set_content_for_tests(&mut self, content: &str) {
        self.with_lock(|guard| {
            guard
                .file
                .set_len(0)
                .expect("could not truncate the pid file");
            guard
                .file
                .write_all(content.as_bytes())
                .expect("could not write into the pid file");
            Ok(())
        })
        .unwrap()
    }

    pub(super) fn with_lock<F, T>(&mut self, call_back: F) -> Result<T, ClaimError>
    where
        F: FnOnce(&mut PidFileLockGuard) -> Result<T, ClaimError>,
    {
        let mut guard = self.lock().map_err(ClaimError::Flock)?;
        call_back(&mut guard)
    }
}

impl<'a> PidFileLockGuard<'a> {
    pub(super) fn read_pid(&mut self) -> Result<Option<u32>, ClaimError> {
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

    pub(super) fn truncate(&mut self) {
        self.file.set_len(0).expect("File truncating should never return an error since the len is 0 and a PidFile only handles a file with write permissions");
    }

    pub(super) fn write_pid(&mut self) -> Result<(), ClaimError> {
        self.truncate();
        self.file
            .seek(io::SeekFrom::Start(0))
            .map_err(ClaimError::RewindFile)?;
        self.file
            .write_all(std::process::id().to_string().as_bytes())
            .map_err(ClaimError::WriteFile)?;
        Ok(())
    }
}
