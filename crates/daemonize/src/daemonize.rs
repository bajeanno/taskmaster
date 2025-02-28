use crate::error::{Error, Result};
use std::{fs, io, os::fd::AsRawFd, process};

#[derive(Default)]
pub struct Daemonize {
    stdin: Option<String>,
    stdout: Option<String>,
    stderr: Option<String>,
}

impl Daemonize {
    pub fn new() -> Self {
        Self::default()
    }

    /// # Errors
    ///
    /// This function will return an error if it fails to fork() / open() / dup2()
    ///
    /// # Safety
    ///
    /// Only call when it is safe to call fork()
    pub unsafe fn start(self) -> Result<()> {
        self.redirect_files()?;
        unsafe { Self::fork() }
    }

    unsafe fn fork() -> Result<()> {
        match unsafe { libc::unistd::fork() } {
            pid if pid < 0 => Err(Error::FailedToFork {
                os_error: io::Error::last_os_error(),
            }),
            pid if pid > 0 => process::exit(0),
            _ => Ok(()),
        }
    }

    fn redirect_files(self) -> Result<()> {
        const DEV_NULL: &str = "/dev/null";

        Self::redirect_file(self.stdin.as_deref().unwrap_or(DEV_NULL), 0, "stdin")?;
        Self::redirect_file(self.stdout.as_deref().unwrap_or(DEV_NULL), 1, "stdout")?;
        Self::redirect_file(self.stderr.as_deref().unwrap_or(DEV_NULL), 2, "stderr")
    }

    fn redirect_file(file_path: &str, fd: i32, redirected_io: &'static str) -> Result<()> {
        let file = fs::OpenOptions::new()
            .read(fd == 0)
            .write(fd != 0)
            .create(true)
            .truncate(false)
            .append(true)
            .open(file_path)
            .map_err(|err| Error::FailedToOpenFile {
                file_path: file_path.to_string(),
                redirected_io,
                err,
            })?;

        if unsafe { libc::unistd::dup2(file.as_raw_fd(), fd) } < 0 {
            return Err(Error::FailedToRedirectFileUsingDup2 {
                file_path: file_path.to_string(),
                redirected_io,
                os_error: io::Error::last_os_error(),
            });
        }
        Ok(())
    }

    pub fn stdin(mut self, stdin: impl Into<String>) -> Self {
        self.stdin = Some(stdin.into());
        self
    }

    pub fn stdout(mut self, stdout: impl Into<String>) -> Self {
        self.stdout = Some(stdout.into());
        self
    }

    pub fn stderr(mut self, stderr: impl Into<String>) -> Self {
        self.stderr = Some(stderr.into());
        self
    }
}
