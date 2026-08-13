mod command;
mod handle;
mod routine;
mod status;
#[cfg(test)]
mod tests;

pub use handle::Handle;

#[allow(unused)]
pub use routine::{Routine, RoutineSpawnError};
pub use status::{NominativeStatus, Status, StatusSender};
use std::fs::{File, OpenOptions};
use std::io::Write;
#[allow(unused)]
use tokio::process::Command;
use tokio::process::{ChildStderr, ChildStdout};
use tokio::sync::Mutex;
use tokio::{io::BufReader, process::Child, sync::mpsc};

#[derive(Debug, Clone, Copy)]
pub enum LogType {
    Stdout,
    Stderr,
}

#[derive(Clone, Debug)]
pub struct Log {
    pub message: String,
    pub process_name: String,
    pub log_type: LogType,
}

impl Log {
    fn new(log_type: LogType, buffer: &[u8], name: &str) -> Self {
        match log_type {
            LogType::Stdout => Log {
                message: format!("{}: {}", name, String::from_utf8_lossy(buffer)),
                process_name: name.to_string(),
                log_type,
            },
            LogType::Stderr => Log {
                message: format!("{}: {}", name, String::from_utf8_lossy(buffer)),
                process_name: name.to_string(),
                log_type,
            },
        }
    }
}

pub type LogReceiver = mpsc::UnboundedReceiver<Log>;
pub type LogSender = mpsc::UnboundedSender<Log>;

pub type StatusReceiver = mpsc::UnboundedReceiver<NominativeStatus>;

pub type KillCommandReceiver = mpsc::Receiver<()>;
pub type KillCommandSender = mpsc::Sender<()>;

pub struct Outputs {
    stdout: BufReader<ChildStdout>,
    stderr: BufReader<ChildStderr>,
}

impl Outputs {
    pub fn new(child: &mut Child) -> Self {
        Self {
            stdout: BufReader::new(
                child
                    .stdout
                    .take()
                    .expect("Child process stdout not captured"),
            ),
            stderr: BufReader::new(
                child
                    .stderr
                    .take()
                    .expect("Child process stderr not captured"),
            ),
        }
    }
}

#[derive(Debug, Default)]
pub enum OutputFile {
    Stdout {
        file: Mutex<File>,
        path: String,
    },
    Stderr {
        file: Mutex<File>,
        path: String,
    },
    #[default]
    None,
}

impl OutputFile {
    pub fn new_stdout(file_path: &str) -> Result<Self, std::io::Error> {
        Ok(Self::Stdout {
            file: Mutex::new(
                OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(file_path)?,
            ),
            path: file_path.to_string(),
        })
    }

    pub fn new_stderr(file_path: &str) -> Result<Self, std::io::Error> {
        Ok(Self::Stderr {
            file: Mutex::new(
                OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(file_path)?,
            ),
            path: file_path.to_string(),
        })
    }

    pub async fn write(&self, log: &Log) {
        match (self, log.log_type) {
            (OutputFile::Stdout { file, path: _ }, LogType::Stdout) => {
                let _ = file.lock().await.write_all(log.message.as_bytes()).inspect_err(|err| {
                    eprintln!("Taskmaster error: {}: Failed to write process stdout output to log file: {err}", log.process_name);
                });
            }
            (OutputFile::Stderr { file, path: _ }, LogType::Stderr) => {
                let _ = file.lock().await.write_all(log.message.as_bytes()).inspect_err(|err| {
                    eprintln!("Taskmaster error: {}: Failed to write process stderr output to log file: {err}", log.process_name);
                });
            }
            (OutputFile::None, _) => { /* Do nothing as there is no file to write output in */ }
            _ => panic!(
                "log function was called with different values for output and log_type, expected same values"
            ),
        }
    }
}

impl PartialEq for OutputFile {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                OutputFile::Stdout {
                    file: _,
                    path: path1,
                },
                OutputFile::Stdout {
                    file: _,
                    path: path2,
                },
            ) => path1 == path2,
            (
                OutputFile::Stderr {
                    file: _,
                    path: path1,
                },
                OutputFile::Stderr {
                    file: _,
                    path: path2,
                },
            ) => path1 == path2,
            (OutputFile::None, OutputFile::None) => true,
            _ => false,
        }
    }
}
