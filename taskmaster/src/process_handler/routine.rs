use super::{Handle, Status};
use crate::parser::program::{AutoRestart, Program};
use std::process::Stdio;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;
use tokio::{fs::File, io::AsyncWriteExt};
use tokio::{
    io::{AsyncBufReadExt, BufReader, Error},
    process::{Child, ChildStderr, ChildStdout},
    select,
    sync::mpsc,
    time::Instant,
};

#[derive(Clone)]
pub enum Log {
    Stdout(String),
    Stderr(String),
}

pub type StatusReceiver = mpsc::Receiver<Status>;
pub type LogReceiver = mpsc::Receiver<Log>;
pub type StatusSender = mpsc::Sender<Status>;

pub struct Outputs {
    stdout: ChildStdout,
    stderr: ChildStderr,
}

#[derive(Error, Debug)]
enum LogError {
    #[error("{0}")]
    SenderDropped(#[from] SendError<Log>),
    #[error("{0}")]
    FileWriteError(#[from] std::io::Error),
}

pub struct Routine {
    status_sender: StatusSender,
    log_sender: mpsc::Sender<Log>,
    config: Program,
    start_attempts: u32,
    status: Status,
    stdout_file: File,
    stderr_file: File,
}

#[allow(dead_code)] //TODO: Remove that
impl Routine {
    pub async fn spawn(config: Program) -> Result<Handle, Error> {
        let (status_sender, status_receiver) = mpsc::channel(100);
        let (log_sender, log_receiver) = mpsc::channel(100);
        let stdout_file = File::create(config.stdout().clone().as_str()).await?;
        let stderr_file = File::create(config.stderr().clone().as_str()).await?;

        let join_handle = tokio::spawn(async move {
            Self {
                stdout_file,
                stderr_file,
                config,
                status_sender,
                log_sender,
                start_attempts: 0,
                status: Status::NotSpawned,
            }
            .routine()
            .await;
        });
        Ok(Handle::new(join_handle, status_receiver, log_receiver))
    }

    async fn routine(mut self) {
        let mut start_time: Instant;
        loop {
            start_time = Instant::now();
            if let Ok(mut child) = self.start().await {
                self.status(Status::Starting).await;
                let outputs = Outputs {
                    stdout: child
                        .stdout
                        .take()
                        .expect("Child process stdout not captured"),
                    stderr: child
                        .stderr
                        .take()
                        .expect("Child process stderr not captured"),
                };
                match self.listen(outputs).await {
                    Ok(()) => {}
                    Err(err) => eprintln!("{err}"),
                }
                self.status(Status::Exited(
                    child.wait().await.expect("error waiting for child"),
                ))
                .await;
            } else {
                self.status(Status::FailedToStart(String::from(
                    "Error spawning sub-process",
                )))
                .await;
            }

            if (self.start_attempts > *self.config.start_retries()
                && start_time.elapsed().as_secs() >= (*self.config.start_time()).into())
                || *self.config.auto_restart() != AutoRestart::True
            {
                break;
            }
        }
    }

    async fn status(&mut self, status: Status) {
        self.status = status.clone();
        self.status_sender
            .send(status)
            .await
            .expect("Receiver was dropped");
    }

    async fn start(&mut self) -> Result<Child, Error> {
        self.start_attempts += 1;
        let child = self
            .config
            .cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        Ok(child)
    }

    /// Sends a log message over the channel and writes it to the appropriate output file.
    ///
    /// This function performs two operations:
    /// 1. Sends the log message through the log channel to any receivers
    /// 2. Writes the log message to the corresponding output file (stdout or stderr)
    async fn log(&mut self, log: Log) -> Result<(), LogError> {
        self.log_sender.send(log.clone()).await?;

        match log {
            Log::Stdout(ref l) => self.stdout_file.write_all(l.as_bytes()).await?,
            Log::Stderr(ref l) => self.stderr_file.write_all(l.as_bytes()).await?,
        }
        Ok(())
    }

    ///  Listens to the outputs of a child process and logs them.
    ///
    ///  This function reads from both stdout and stderr streams of a child process,
    ///  splitting the output by newlines and logging each line as it arrives.
    ///
    ///  The function uses `tokio::select!` to concurrently read from both streams,
    ///  continuing until either stream is exhausted (read returns 0 bytes) or an
    ///  error occurs. After the main loop exits, it flushes any remaining data that
    ///  may not have been terminated by a newline character.
    ///
    ///  # Arguments
    ///
    ///  * `outputs` - An `Outputs` struct containing the stdout and stderr handles
    ///    from the child process.
    ///
    ///  # Panics
    ///
    ///  Will panic if the log sender has been dropped, which would indicate a
    ///  critical failure in the channel communication.
    async fn listen(&mut self, outputs: Outputs) -> Result<(), LogError> {
        let mut stdout = BufReader::new(outputs.stdout);
        let mut stderr = BufReader::new(outputs.stderr);
        let mut stdout_output;
        let mut stderr_output;
        loop {
            stdout_output = Vec::new();
            stderr_output = Vec::new();
            select! {
                read_result = stdout.read_until(b'\n', &mut stdout_output) => {
                    match read_result {
                        Ok(read_result) => {
                            if read_result == 0 { break; }
                        },
                        Err(err) => {
                            eprintln!("Error encountered while reading stdout: {err}");
                            break;
                        },
                    }
                },
                read_result = stderr.read_until(b'\n', &mut stderr_output) => {
                    match read_result {
                        Ok(read_result) => {
                            if read_result == 0 { break; }
                        },
                        Err(err) => {
                            eprintln!("Error encountered while reading stderr: {err}");
                            break;
                        },
                    }
                },
                else => break,
            }

            if !stdout_output.is_empty() {
                self.log(Log::Stdout(
                    String::from_utf8_lossy(&stdout_output).to_string(),
                ))
                .await?;
            }
            if !stderr_output.is_empty() {
                self.log(Log::Stderr(
                    String::from_utf8_lossy(&stderr_output).to_string(),
                ))
                .await?;
            }
        }
        if !stdout_output.is_empty() {
            self.log(Log::Stdout(
                String::from_utf8_lossy(&stdout_output).to_string(),
            ))
            .await?;
        }
        if !stderr_output.is_empty() {
            self.log(Log::Stderr(
                String::from_utf8_lossy(&stderr_output).to_string(),
            ))
            .await?;
        }
        Ok(())
    }
}
