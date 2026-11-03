use libc::sys::file::{self, LOCK_EX, flock};
use std::{
    fs::File,
    ops::{Deref, DerefMut},
    os::fd::AsRawFd,
};

#[derive(Debug)]
pub struct FileLock {
    file: File,
}

impl<'a> FileLock {
    pub fn new(file: File) -> Self {
        Self { file }
    }

    pub fn lock(&'a self) -> Result<FileLockGuard<'a>, std::io::Error> {
        if unsafe { flock(self.file.as_raw_fd(), LOCK_EX) == -1 } {
            return Err(std::io::Error::last_os_error());
        }
        Ok(FileLockGuard { file: &self.file })
    }

    pub fn fd(&self) -> i32 {
        self.file.as_raw_fd()
    }
}

#[derive(Debug)]
pub struct FileLockGuard<'a> {
    file: &'a File,
}

impl<'a> Drop for FileLockGuard<'a> {
    fn drop(&mut self) {
        unsafe { file::flock(self.file.as_raw_fd(), file::LOCK_UN) };
    }
}

impl<'a> Deref for FileLockGuard<'a> {
    type Target = &'a File;

    fn deref(&self) -> &Self::Target {
        &self.file
    }
}

impl<'a> DerefMut for FileLockGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.file
    }
}
