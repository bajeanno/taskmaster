use super::{Handle, Status};
use crate::parser::program::{AutoRestart, Program};
use std::panic;
use std::process::Stdio;
use tokio::io::AsyncBufRead;
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

enum OutputFile {
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
        const BUFFER_SIZE: usize = 100; // 100 is a temporary value
        let (status_sender, status_receiver) = mpsc::channel(BUFFER_SIZE);
        let (log_sender, log_receiver) = mpsc::channel(BUFFER_SIZE);
        let stdout_file = Mutex::new(OutputFile::Stdout(File::create(config.stdout()).await?));
        let stderr_file = Mutex::new(OutputFile::Stderr(File::create(config.stderr()).await?));

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

    async fn routine(mut self, stdout_file: &Mutex<OutputFile>, stderr_file: &Mutex<OutputFile>) {
        loop {
            let start_time = Instant::now();

            self.run_program(stdout_file, stderr_file).await;

            if !self.should_try_restart(start_time) {
                break;
            }
        }
    }

    async fn run_program(
        &mut self,
        stdout_file: &Mutex<OutputFile>,
        stderr_file: &Mutex<OutputFile>,
    ) {
        match self.child_spawn().await {
            Ok(child) => {
                self.handle_running_child(child, stdout_file, stderr_file)
                    .await
            }
            Err(err) => {
                self.status(Status::FailedToSpawn {
                    error_message: format!("Error spawning sub-process: {err}"),
                })
                .await
            }
        }
    }

    async fn handle_running_child(
        &mut self,
        mut child: Child,
        stdout_file: &Mutex<OutputFile>,
        stderr_file: &Mutex<OutputFile>,
    ) {
        self.status(Status::Starting).await;

        let outputs = Outputs::new(&mut child);

        match *self.config.start_time() {
            0 => {}
            start_time => {
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(start_time as u64)) => { }

                    exit_status = child.wait() => {
                        self.status(Status::FailedToInit {
                            error_message: "Process crashed before finishing initialization".to_owned(),
                            exit_code: exit_status
                                .expect("Error getting exit status from subprocess")
                                .code()
                                .expect("unable to retrieve exit code")
                                as u8
                        }).await;
                        return;
                    }
                }
            }
        }

        self.status(Status::Running).await;
        // TODO need to listen also before running is validated
        self.listen(outputs, stdout_file, stderr_file).await;
        //TODO: Would be nice to share the exit code inside the enum
        self.status(Status::Exited(
            child.wait().await.expect("error waiting for child"),
        ))
        .await;
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
    fn should_try_restart(&mut self, start_time: Instant) -> bool {
        let started_properly = start_time.elapsed().as_secs() >= (*self.config.start_time()).into();

        if started_properly {
            self.start_attempts = 0;

            if *self.config.auto_restart() == AutoRestart::False {
                return false;
            }

            if *self.config.auto_restart() == AutoRestart::OnFailure && self.is_expected_status() {
                return false;
            }

            true
        } else {
            // started_properly == false

            if self.start_attempts >= *self.config.start_retries() {
                return false;
            }

            true
        }
    }

    fn is_expected_status(&self) -> bool {
        if let Status::Exited(exit_status) = &self.status {
            match exit_status.code() {
                Some(exit_status) => self.config.exit_codes().contains(&(exit_status as u8)),
                None => false,
            }
        } else {
            false
        }
    }

    /// Registers status to Routine struct and sends it to backend thread (task manager)
    ///
    /// # Arguments
    /// * status - The `Status` enum that describes the current status of the subprocess handled by Routine
    async fn status(&mut self, status: Status) {
        self.status = status.clone();
        self.status_sender
            .send(status)
            .await
            .expect("Receiver was dropped");
    }

    /// Spawns the child and upgrades the start_attempts counter
    async fn child_spawn(&mut self) -> Result<Child, Error> {
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
        &self,
        outputs: Outputs,
        stdout_file: &Mutex<OutputFile>,
        stderr_file: &Mutex<OutputFile>,
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
                self.config.name()
            ),
            listen_and_log(
                stderr,
                self.log_sender.clone(),
                &mut stderr_file_mutex_guard,
                self.config.name()
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
                let log = match output_file {
                    OutputFile::Stdout(_) => Log {
                        message: String::from_utf8_lossy(&buffer).to_string(),
                        program_name: name.to_string(),
                        log_type: LogType::Stdout,
                    },
                    OutputFile::Stderr(_) => Log {
                        message: String::from_utf8_lossy(&buffer).to_string(),
                        program_name: name.to_string(),
                        log_type: LogType::Stderr,
                    },
                };
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
