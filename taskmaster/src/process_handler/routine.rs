use super::{Handle, Status};
use crate::parser::program::{AutoRestart, Program};
use std::process::Stdio;
use tokio::io::AsyncBufRead;
use tokio::{fs::File, io::AsyncWriteExt};
use tokio::{
    io::{AsyncBufReadExt, BufReader, Error},
    process::{Child, ChildStderr, ChildStdout},
    sync::{Mutex, mpsc},
    time::Instant,
};

#[derive(Clone, Debug)]
pub enum Log {
    Stdout(String, String),
    Stderr(String, String),
}

pub type StatusReceiver = mpsc::Receiver<Status>;
pub type LogReceiver = mpsc::Receiver<Log>;
pub type StatusSender = mpsc::Sender<Status>;
pub type LogSender = mpsc::Sender<Log>;

pub struct Outputs {
    stdout: ChildStdout,
    stderr: ChildStderr,
}

impl Outputs {
    pub fn new(child: &mut Child) -> Self {
        Self {
            stdout: child
                .stdout
                .take()
                .expect("Child process stdout not captured"),
            stderr: child
                .stderr
                .take()
                .expect("Child process stderr not captured"),
        }
    }
}

enum OutputType {
    Stdout(File),
    Stderr(File),
}

// #[derive(Error, Debug)]
// enum LogError {
//     #[error("{0}")]
//     FileWriteError(#[from] std::io::Error),
// }

pub struct Routine {
    status_sender: StatusSender,
    log_sender: LogSender,
    config: Program,
    start_attempts: u32,
    status: Status,
}

#[allow(dead_code)] //TODO: Remove that
impl Routine {
    pub async fn spawn(config: Program) -> Result<Handle, Error> {
        let (status_sender, status_receiver) = mpsc::channel(100);
        let (log_sender, log_receiver) = mpsc::channel(100);
        let stdout_file = Mutex::new(OutputType::Stdout(
            File::create(config.stdout().clone().as_str()).await?,
        ));
        let stderr_file = Mutex::new(OutputType::Stderr(
            File::create(config.stderr().clone().as_str()).await?,
        ));

        let join_handle = tokio::spawn(async move {
            Self {
                config,
                status_sender,
                log_sender,
                start_attempts: 0,
                status: Status::NotSpawned,
            }
            .routine(&stdout_file, &stderr_file)
            .await;
        });
        Ok(Handle::new(join_handle, status_receiver, log_receiver))
    }

    async fn routine(mut self, stdout_file: &Mutex<OutputType>, stderr_file: &Mutex<OutputType>) {
        let mut start_time: Instant;
        loop {
            start_time = Instant::now();
            if let Ok(mut child) = self.start().await {
                self.status(Status::Starting).await;
                let outputs = Outputs::new(&mut child);

                self.listen(
                    outputs,
                    stdout_file,
                    stderr_file,
                    &self.config.name().clone(),
                )
                .await;
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
    async fn listen(
        &mut self,
        outputs: Outputs,
        stdout_file: &Mutex<OutputType>,
        stderr_file: &Mutex<OutputType>,
        name: &str,
    ) {
        let stdout = BufReader::new(outputs.stdout);
        let stderr = BufReader::new(outputs.stderr);

        let mut stdout_file_mutex_guard = stdout_file.lock().await;
        let mut stderr_file_mutex_guard = stderr_file.lock().await;

        tokio::join!(
            listen_and_log(
                stdout,
                self.log_sender.clone(),
                &mut stdout_file_mutex_guard,
                name
            ),
            listen_and_log(
                stderr,
                self.log_sender.clone(),
                &mut stderr_file_mutex_guard,
                name
            ),
        );
    }
}

/// Sends a log message over the channel and writes it to the appropriate output file.
/// This function performs two operations:
/// 1. Sends the log message through the log channel to any receivers
/// 2. Writes the log message to the corresponding output file (stdout or stderr)
async fn dispatch_log(log: Log, log_sender: &mut LogSender, output: &mut OutputType) {
    match output {
        OutputType::Stdout(file) => match log {
            Log::Stdout(ref l, ref name) => {
                let _ = file.write_all(l.as_bytes()).await.inspect_err(|err| {
                    eprintln!("Taskmaster error: {name}: Failed to write process stdout output to log file: {err}");
                });
            }
            _ => panic!(
                "log function was called with the file for stdout, but the log was an stderr"
            ),
        },
        OutputType::Stderr(file) => match log {
            Log::Stderr(ref l, ref name) => {
                let _ = file.write_all(l.as_bytes()).await.inspect_err(|err| {
                    eprintln!("Taskmaster error: {name}: Failed to write process stderr output to log file: {err}");
                });
            }
            _ => panic!(
                "log function was called with the file for stderr, but the log was an stdout"
            ),
        },
    };
    log_sender
        .send(log.clone())
        .await
        .expect("Taskmaster error: {log.1}: Log receiver was dropped");
}

async fn listen_and_log<R: AsyncBufRead + Unpin>(
    mut output: R,
    mut sender: LogSender,
    output_type: &mut OutputType,
    name: &str,
) {
    loop {
        let mut buffer = Vec::new();
        let bytes_read = output.read_until(b'\n', &mut buffer).await;

        match bytes_read {
            Ok(0) => break,
            Ok(_) => {
                let log = match output_type {
                    OutputType::Stderr(_) => Log::Stderr(
                        String::from_utf8_lossy(&buffer).to_string(),
                        name.to_string(),
                    ),
                    OutputType::Stdout(_) => Log::Stdout(
                        String::from_utf8_lossy(&buffer).to_string(),
                        name.to_string(),
                    ),
                };
                dispatch_log(log, &mut sender, output_type).await;
            }
            Err(err) => {
                eprintln!(
                    "Taskmaster error: {name}: Error encountered while reading stderr: {err}"
                );
                break;
            }
        }
    }
}
