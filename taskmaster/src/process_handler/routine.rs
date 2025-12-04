use super::{Handle, Status};
use crate::parser::program::{AutoRestart, Program};
use std::process::Stdio;
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
    pub fn spawn(config: Program) -> Result<Handle, Error> {
        let (sender, receiver) = mpsc::channel(100);
        let (log_sender, log_receiver) = mpsc::channel(100);

        let join_handle = tokio::spawn(async move {
            Self {
                stdout_file: File::create(config.stdout().clone().as_str())
                    .await
                    .expect("Failed to create stdout log file"),
                stderr_file: File::create(config.stderr().clone().as_str())
                    .await
                    .expect("Failed to create stderr log file"),
                config,
                status_sender: sender,
                log_sender,
                start_attempts: 0,
                status: Status::NotSpawned,
            }
            .routine()
            .await;
        });
        Ok(Handle::new(join_handle, receiver, log_receiver))
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
                self.listen(outputs).await;
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
    async fn log(&mut self, log: Log) {
        self.log_sender
            .send(log.clone())
            .await
            .expect("Log receiver dropped");
        match log {
            Log::Stdout(ref l) => {
                self.stdout_file
                    .write_all(l.as_bytes())
                    .await
                    .expect("Failed to write to stdout log file");
            }
            Log::Stderr(ref l) => {
                self.stderr_file
                    .write_all(l.as_bytes())
                    .await
                    .expect("Failed to write to stderr log file");
            }
        }
    }

    async fn listen(&mut self, outputs: Outputs) {
        let mut stdout = BufReader::new(outputs.stdout);
        let mut stderr = BufReader::new(outputs.stderr);

        loop {
            let mut stdout_output = Vec::new();
            let mut stderr_output = Vec::new();
            let log;

            select! {
                read_result = stdout.read_until(b'\n', &mut stdout_output) => {
                    match read_result {
                        Ok(read_result) => {
                            if read_result == 0 { break; }
                            log = Log::Stdout(String::from_utf8_lossy(&stdout_output).to_string());
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
                            log = Log::Stderr(String::from_utf8_lossy(&stderr_output).to_string());
                        },
                        Err(err) => {
                            eprintln!("Error encountered while reading stderr: {err}");
                            break;
                        },
                    }
                },
                else => break,
            }

            if !stdout_output.is_empty() || !stderr_output.is_empty() {
                self.log(log).await;
            }
        }
    }
}
