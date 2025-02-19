use std::error::Error;
use std::fmt::Display;
use std::fs;
use std::io;
use std::os::fd::AsRawFd;
use std::process;

#[derive(Default)]
pub struct Daemonize<'a> {
    stdin: Option<&'a str>,
    stdout: Option<&'a str>,
    stderr: Option<&'a str>,
}

#[derive(Debug)]
pub enum DaemonizeError {
    FailedToFork,
    FailedToRedirectFile(FailedToRedirectFile),
}

impl Display for DaemonizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for DaemonizeError {}

#[derive(Debug)]
pub enum FailedToRedirectFile {
    OpenError((&'static str, io::Error)),
    Dup2Error((&'static str, io::Error)),
}

struct DaemonizeData<'a> {
    stdin: &'a str,
    stdout: &'a str,
    stderr: &'a str,
}

impl<'a> Daemonize<'a> {
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
    pub unsafe fn start(self) -> Result<(), DaemonizeError> {
        self.redirect_files().map_err(DaemonizeError::FailedToRedirectFile)?;
        unsafe { Self::fork() }
    }

    unsafe fn fork() -> Result<(), DaemonizeError> {
        match unsafe { libc::unistd::fork() } {
            pid if pid < 0 => Err(DaemonizeError::FailedToFork),
            pid if pid > 0 => process::exit(0),
            _ => Ok(()),
        }
    }

    fn redirect_files(self) -> Result<(), FailedToRedirectFile> {
        let data: DaemonizeData = self.into();

        Self::redirect_file(data.stdin, 0, "stdin")?;
        Self::redirect_file(data.stdout, 1, "stdout")?;
        Self::redirect_file(data.stderr, 2, "stderr")
    }

    fn redirect_file(
        file: &str,
        fd: i32,
        io_name: &'static str,
    ) -> Result<(), FailedToRedirectFile> {
        let file = fs::OpenOptions::new()
            .read(fd == 0)
            .write(fd != 0)
            .create(true)
            .truncate(false)
            .append(true)
            .open(file)
            .map_err(|err| FailedToRedirectFile::OpenError((io_name, err)))?;

        match unsafe { libc::unistd::dup2(file.as_raw_fd(), fd) } {
            res if res < 0 => Err(FailedToRedirectFile::Dup2Error((
                io_name,
                io::Error::last_os_error(),
            ))),
            _ => Ok(()),
        }
    }

    pub fn stdin(mut self, stdin: &'a str) -> Self {
        self.stdin = Some(stdin);
        self
    }

    pub fn stdout(mut self, stdout: &'a str) -> Self {
        self.stdout = Some(stdout);
        self
    }

    pub fn stderr(mut self, stderr: &'a str) -> Self {
        self.stderr = Some(stderr);
        self
    }
}

impl<'a> From<Daemonize<'a>> for DaemonizeData<'a> {
    fn from(daemon: Daemonize<'a>) -> Self {
        Self {
            stdin: daemon.stdin.unwrap_or("/dev/null"),
            stdout: daemon.stdout.unwrap_or("/dev/null"),
            stderr: daemon.stderr.unwrap_or("/dev/null"),
        }
    }
}
