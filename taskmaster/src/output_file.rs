use std::{
    fs::{File, OpenOptions},
    io::Write,
};

use tokio::sync::Mutex;

use crate::process_handler::{Log, LogType};

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
