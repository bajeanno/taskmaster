use std::{fs, io, os::fd::AsRawFd};

use libc::sys::types::Pid;

#[derive(Debug, thiserror::Error)]
pub enum OutsideDaemonError {
    #[error("Fork failed")]
    FailedToFork(#[source] io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum InsideDaemonError {
    #[error("Failed to open file '{file_path}' while redirecting {redirected_io}")]
    FailedToOpenFile {
        file_path: String,
        redirected_io: &'static str,
        source: io::Error,
    },
    #[error("Failed to redirect {redirected_io} to '{file_path}'")]
    FailedToRedirectFileUsingDup2 {
        file_path: String,
        redirected_io: &'static str,
        source: io::Error,
    },
}

#[derive(Default)]
pub struct Daemonize {
    stdin: Option<String>,
    stdout: Option<String>,
    stderr: Option<String>,
}

#[must_use = "InsideDaemon and OutsideDaemon variants may contain an error"]
pub enum Daemonized<InsideDaemon, OutsideDaemon> {
    InsideDaemon(InsideDaemon),
    OutsideDaemon(OutsideDaemon),
}

impl<InsideDaemon, OutsideDaemon> Daemonized<InsideDaemon, OutsideDaemon> {
    pub fn map_inside_daemon<NewInsideDaemon>(
        self,
        f: impl FnOnce(InsideDaemon) -> NewInsideDaemon,
    ) -> Daemonized<NewInsideDaemon, OutsideDaemon> {
        match self {
            Daemonized::InsideDaemon(tmp) => Daemonized::InsideDaemon(f(tmp)),
            Daemonized::OutsideDaemon(tmp) => Daemonized::OutsideDaemon(tmp),
        }
    }

    pub fn map_outside_daemon<NewOutsideDaemon>(
        self,
        f: impl FnOnce(OutsideDaemon) -> NewOutsideDaemon,
    ) -> Daemonized<InsideDaemon, NewOutsideDaemon> {
        match self {
            Daemonized::InsideDaemon(tmp) => Daemonized::InsideDaemon(tmp),
            Daemonized::OutsideDaemon(tmp) => Daemonized::OutsideDaemon(f(tmp)),
        }
    }
}

pub type DefaultDaemonized =
    Daemonized<Result<(), InsideDaemonError>, Result<Pid, OutsideDaemonError>>;

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
    pub unsafe fn start(self) -> DefaultDaemonized {
        unsafe { Self::fork() }.map_inside_daemon(|res| res.and_then(|()| self.redirect_files()))
    }

    unsafe fn fork() -> DefaultDaemonized {
        match unsafe { libc::unistd::fork() } {
            pid if pid < 0 => Daemonized::OutsideDaemon(Err(OutsideDaemonError::FailedToFork(
                io::Error::last_os_error(),
            ))),
            pid if pid > 0 => Daemonized::OutsideDaemon(Ok(pid)),
            _ => Daemonized::InsideDaemon(Ok(())),
        }
    }

    fn redirect_files(self) -> Result<(), InsideDaemonError> {
        const DEV_NULL: &str = "/dev/null";

        Self::redirect_file(self.stdin.as_deref().unwrap_or(DEV_NULL), 0, "stdin")?;
        Self::redirect_file(self.stdout.as_deref().unwrap_or(DEV_NULL), 1, "stdout")?;
        Self::redirect_file(self.stderr.as_deref().unwrap_or(DEV_NULL), 2, "stderr")
    }

    fn redirect_file(
        file_path: &str,
        fd: i32,
        redirected_io: &'static str,
    ) -> Result<(), InsideDaemonError> {
        let file = fs::OpenOptions::new()
            .read(fd == 0)
            .write(fd != 0)
            .create(true)
            .truncate(false)
            .append(true)
            .open(file_path)
            .map_err(|source| InsideDaemonError::FailedToOpenFile {
                file_path: file_path.to_string(),
                redirected_io,
                source,
            })?;

        if unsafe { libc::unistd::dup2(file.as_raw_fd(), fd) } < 0 {
            return Err(InsideDaemonError::FailedToRedirectFileUsingDup2 {
                file_path: file_path.to_string(),
                redirected_io,
                source: io::Error::last_os_error(),
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
