use super::{Handle, Status};
use crate::parser::program::{AutoRestart, Program};
use std::process::Stdio;
use tokio::{fs::File, io::AsyncWriteExt};
#[allow(unused)] //TODO: remove that
use tokio::{
    io::{AsyncBufReadExt, BufReader, Error},
    process::{Child, ChildStderr, ChildStdout, Command},
    select,
    sync::mpsc,
    time::{Duration, Instant, sleep},
};

pub type StatusReceiver = mpsc::Receiver<Status>;
pub type StatusSender = mpsc::Sender<Status>;

pub struct Outputs {
    stdout: ChildStdout,
    stderr: ChildStderr,
}

#[allow(dead_code)] //TODO: Remove that
pub struct Routine {
    sender: StatusSender,
    log_sender: mpsc::Sender<String>,
    config: Program,
    start_attempts: u32,
    outputs: Option<Outputs>,
    child: Option<Child>,
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
                stdout_file: File::create(
                    config
                        .stdout()
                        .clone()
                        .as_str()
                )
                .await
                .expect("Failed to create stdout log file"),
                stderr_file: File::create(
                    config
                        .stderr()
                        .clone()
                        .as_str()
                )
                .await
                .expect("Failed to create stderr log file"),
                config,
                sender,
                log_sender,
                start_attempts: 0,
                outputs: None,
                child: None,
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
                self.listen().await;
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
                && start_time.elapsed().as_secs() >= self.config.start_time().clone().into())
                || *self.config.auto_restart() != AutoRestart::True
            {
                break;
            }
        }
    }

    async fn status(&mut self, status: Status) {
        self.status = status.clone();
        self.sender
            .send(status)
            .await
            .expect("Receiver was dropped");
    }

    async fn start(&mut self) -> Result<Child, Error> {
        self.start_attempts += 1;

        let mut child = self
            .config
            .cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        self.outputs = Some(Outputs {
            stdout: child.stdout.take().expect("Failed to open stdout"),
            stderr: child.stderr.take().expect("Failed to open stderr"),
        });
        Ok(child)
    }

    //TODO: check if there's not a better way
    async fn log_stdout(&mut self, log: &String) {
        self.log_sender.send(log.clone()).await.expect("Log receiver dropped");
        self.stdout_file.write_all(log.as_bytes()).await.expect("Failed to write to stdout log file");
    }

    async fn log_stderr(&mut self, log: &String){
        self.log_sender.send(log.clone()).await.expect("Log receiver dropped");
        self.stdout_file.write_all(log.as_bytes()).await.expect("Failed to write to stdout log file");
    }

    async fn listen(&mut self) {
        let outputs: Outputs = self.outputs.take().expect("Outputs should not be None");
        let mut stdout = BufReader::new(outputs.stdout);
        let mut stderr = BufReader::new(outputs.stderr);

        loop {
            let mut stdout_output = Vec::new();
            let mut stderr_output = Vec::new();
            let output;

            select! {
                Ok(read_result) = stdout.read_until(b'\n', &mut stdout_output) => {
                    if read_result == 0 { break; }
                    output = String::from_utf8_lossy(&stdout_output).to_string();
                },
                Ok(read_result) = stderr.read_until(b'\n', &mut stderr_output) => {
                    if read_result == 0 { break; }
                    output = String::from_utf8_lossy(&stderr_output).to_string();
                },
                else => break,
            }

            if !stdout_output.is_empty() {
                self.log_stdout(&output).await;
            }
            if !stderr_output.is_empty() {
                self.log_stderr(&output).await;
            }
        }
    }
}
