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

impl<'file> FileLock {
    pub fn new(file: File) -> Self {
        Self { file }
    }

    pub fn lock(&'file self) -> Result<FileLockGuard<'file>, std::io::Error> {
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
pub struct FileLockGuard<'file> {
    file: &'file File,
}

impl<'file> Drop for FileLockGuard<'file> {
    fn drop(&mut self) {
        unsafe { file::flock(self.file.as_raw_fd(), file::LOCK_UN) };
    }
}

impl<'file> Deref for FileLockGuard<'file> {
    type Target = &'file File;

    fn deref(&self) -> &Self::Target {
        &self.file
    }
}

impl<'file> DerefMut for FileLockGuard<'file> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.file
    }
}
