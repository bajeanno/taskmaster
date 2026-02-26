use super::{Handle, Status, command};
use crate::config::program::{AutoRestart, Program};
use std::process::Stdio;
use std::sync::Arc;
use thiserror::Error;
use tokio::io::AsyncBufRead;
use tokio::process::Command;
use tokio::{fs::File, io::AsyncWriteExt};
use tokio::{
    io::{AsyncBufReadExt, BufReader, Error},
    process::{Child, ChildStderr, ChildStdout},
    sync::{Mutex, mpsc},
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
pub enum LogType {
    Stdout,
    Stderr,
}

#[derive(Clone, Debug)]
pub struct Log {
    pub message: String,
    pub program_name: String,
    pub log_type: LogType,
}

impl Log {
    fn new(output_file: &OutputFile, buffer: &[u8], name: &str) -> Self {
        match output_file {
            OutputFile::Stdout(_) => Log {
                message: String::from_utf8_lossy(buffer).to_string(),
                program_name: name.to_string(),
                log_type: LogType::Stdout,
            },
            OutputFile::Stderr(_) => Log {
                message: String::from_utf8_lossy(buffer).to_string(),
                program_name: name.to_string(),
                log_type: LogType::Stderr,
            },
        }
    }
}

pub type StatusReceiver = mpsc::Receiver<Status>;
pub type LogReceiver = mpsc::Receiver<Log>;
pub type StatusSender = mpsc::Sender<Status>;
pub type LogSender = mpsc::Sender<Log>;

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

enum OutputFile {
    Stdout(File),
    Stderr(File),
}

pub struct Routine {
    status_sender: StatusSender,
    log_sender: LogSender,
    config: Program,
    start_attempts: u32,
    command: Command,
}

#[derive(Error, Debug)]
pub enum RoutineSpawnError {
    #[error("{0}")]
    Open(#[from] std::io::Error),
    #[error("{0}")]
    Command(#[from] command::CommandError),
}

#[allow(dead_code)] //TODO: Remove that
impl Routine {
    pub async fn spawn(config: Program) -> Result<Handle, RoutineSpawnError> {
        const BUFFER_SIZE: usize = 100; // 100 is a temporary value
        let (status_sender, status_receiver) = mpsc::channel(BUFFER_SIZE);
        let (log_sender, log_receiver) = mpsc::channel(BUFFER_SIZE);
        let stdout_file = Arc::new(Mutex::new(OutputFile::Stdout(
            File::create(config.stdout()).await?,
        )));
        let stderr_file = Arc::new(Mutex::new(OutputFile::Stderr(
            File::create(config.stderr()).await?,
        )));
        let command = command::create_command(&config)?;

        let join_handle = tokio::spawn(async move {
            Self {
                config,
                status_sender,
                log_sender,
                start_attempts: 0,
                command,
            }
            .routine(stdout_file, stderr_file)
            .await;
        });
        Ok(Handle::new(join_handle, status_receiver, log_receiver))
    }

    async fn routine(
        mut self,
        stdout_file: Arc<Mutex<OutputFile>>,
        stderr_file: Arc<Mutex<OutputFile>>,
    ) {
        loop {
            let start_time = Instant::now();

            self.send_new_status_to_task_manager(Status::Starting).await;
            let status = self
                .run_program(Arc::clone(&stdout_file), Arc::clone(&stderr_file))
                .await;

            let should_try_restart = self.should_try_restart(start_time, &status);

            self.send_new_status_to_task_manager(status).await;

            if !should_try_restart {
                break;
            }
        }
    }

    async fn run_program(
        &mut self,
        stdout_file: Arc<Mutex<OutputFile>>,
        stderr_file: Arc<Mutex<OutputFile>>,
    ) -> Status {
        match self.child_spawn().await {
            Ok(child) => {
                self.send_new_status_to_task_manager(Status::Running).await;
                self.handle_running_child(child, stdout_file, stderr_file)
                    .await
            }
            Err(err) => Status::FailedToSpawn(err),
        }
    }

    async fn handle_running_child(
        &self,
        mut child: Child,
        stdout_file: Arc<Mutex<OutputFile>>,
        stderr_file: Arc<Mutex<OutputFile>>,
    ) -> Status {
        let outputs = Outputs::new(&mut child);
        let listen_task = tokio::spawn(Self::listen(
            outputs,
            stdout_file,
            stderr_file,
            self.log_sender.clone(),
            self.config.name().clone(),
        ));

        match *self.config.start_time() {
            0 => {}
            start_time => {
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(start_time as u64)) => { }

                    exit_status = child.wait() => {
                        listen_task.await.expect("Listen task panicked");
                        return Status::ErrorDuringStartup {
                            exit_code: exit_status
                                .expect("Failed to get exit status")
                                .code()
                                .expect("Failed to get exit code") as u8
                        };
                    }
                }
            }
        }

        self.send_new_status_to_task_manager(Status::Running).await;
        listen_task.await.expect("Listen task panicked");
        Status::Exited(child.wait().await.expect("error waiting for child"))
    }

    /// Condition for restart:
    /// - The programmed failed to start (i.e. it crashed before `config.start_time` seconds
    ///   elapsed):
    ///   - We already attempted to start the program `config.start_retries` times (note that the
    ///     attempted start count is reset whenever the program starts successfully):
    ///     returns false (we don't want to retry)
    ///   - otherwise return true (we want to retry)
    ///
    /// - The program started properly:
    ///   - `config.auto_restart` is `false`: Return false (we don't want to restart)
    ///   - `config.auto_restart` is `unexpected` and the exit status is in `config.exitcodes`: Return false (we don't want to restart)
    ///   - otherwise return true (we want to restart)
    ///
    fn should_try_restart(&mut self, start_time: Instant, status: &Status) -> bool {
        let started_properly = start_time.elapsed().as_secs() >= (*self.config.start_time()).into();

        if started_properly {
            self.start_attempts = 0;

            match *self.config.auto_restart() {
                AutoRestart::False => false,
                AutoRestart::OnFailure => !self.is_expected_status(status),
                AutoRestart::True => true,
            }
        } else {
            if self.start_attempts >= *self.config.start_retries() {
                return false;
            }

            true
        }
    }

    fn is_expected_status(&self, status: &Status) -> bool {
        if let Status::Exited(exit_status) = status {
            match exit_status.code() {
                Some(exit_status) => self.config.exit_codes().contains(&(exit_status as u8)),
                None => false,
            }
        } else {
            false
        }
    }

    async fn send_new_status_to_task_manager(&self, status: Status) {
        self.status_sender
            .send(status)
            .await
            .expect("Receiver was dropped");
    }

    /// Spawns the child and upgrades the start_attempts counter
    async fn child_spawn(&mut self) -> Result<Child, Error> {
        self.start_attempts += 1;
        let child = self
            .command
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
        outputs: Outputs,
        stdout_file: Arc<Mutex<OutputFile>>,
        stderr_file: Arc<Mutex<OutputFile>>,
        log_sender: LogSender,
        program_name: String,
    ) {
        let stdout = outputs.stdout;
        let stderr = outputs.stderr;

        let mut stdout_file_mutex_guard = stdout_file.lock().await;
        let mut stderr_file_mutex_guard = stderr_file.lock().await;

        tokio::join!(
            listen_and_log(
                stdout,
                log_sender.clone(),
                &mut stdout_file_mutex_guard,
                &program_name
            ),
            listen_and_log(
                stderr,
                log_sender,
                &mut stderr_file_mutex_guard,
                &program_name
            ),
        );
    }
}

/// Sends a log message over the channel and writes it to the appropriate output file.
/// This function performs two operations:
/// - Write the log message to the corresponding output file (stdout or stderr)
/// - Send the log message through the log channel to any receivers
///
/// # Arguments
///
/// * `log` - A `Log` struct containing the log type, the task's name and the log itself
/// * `log_sender` - A `mpsc::Sender<Log>` to send log to the manager coroutine
/// * `output` - A `OutputFile` enum that contains the file to write in
///
/// # Panics
///
/// Will panic if the `OutputFile` and the `LogType` enums are not accorded.
/// That should never happen because those structs are both constructed side by side.
///
async fn dispatch_log(log: Log, log_sender: &mut LogSender, output: &mut OutputFile) {
    match (output, &log.log_type) {
        (OutputFile::Stdout(file), LogType::Stdout) => {
            let _ = file.write_all(log.message.as_bytes()).await.inspect_err(|err| {
                eprintln!("Taskmaster error: {}: Failed to write process stdout output to log file: {err}", log.program_name);
            });
        }
        (OutputFile::Stderr(file), LogType::Stderr) => {
            let _ = file.write_all(log.message.as_bytes()).await.inspect_err(|err| {
                eprintln!("Taskmaster error: {}: Failed to write process stderr output to log file: {err}", log.program_name);
            });
        }
        _ => panic!(
            "log function was called with different values for output and log_type, expected same values"
        ),
    }
    log_sender
        .send(log.clone())
        .await
        .inspect_err(|_| {
            eprintln!(
                "Taskmaster error: {}: Log receiver was dropped",
                log.program_name
            )
        })
        .unwrap()
}

async fn listen_and_log<R: AsyncBufRead + Unpin>(
    mut output: R,
    mut sender: LogSender,
    output_file: &mut OutputFile,
    name: &str,
) {
    loop {
        let mut buffer = Vec::new();
        let bytes_read = output.read_until(b'\n', &mut buffer).await;

        match bytes_read {
            Ok(0) => break,
            Ok(_) => {
                let log = Log::new(output_file, &buffer, name);
                dispatch_log(log, &mut sender, output_file).await;
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
