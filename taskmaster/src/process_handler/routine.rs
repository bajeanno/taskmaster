use super::KillCommandReceiver;
use super::LogSender;
use super::{Handle, NominativeStatus, Status, StatusSender, command};
use crate::config::{AutoRestart, ProgramConfig};
use crate::process_handler::{Log, LogType, OutputFile, Outputs};
use libc::signal::kill;
use libc::unistd::{mode_t, umask};
use signal::Signal;
use std::io::Write;
use std::panic;
use std::process::Stdio;
use std::sync::{Arc, LazyLock};
use thiserror::Error;
use tokio::process::Command;
use tokio::sync::mpsc::UnboundedSender;
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, Error},
    process::Child,
    sync::{Mutex, mpsc},
    time::Duration,
};

pub struct Routine {
    status_sender: StatusSender,
    log_sender: LogSender,
    kill_command_receiver: KillCommandReceiver,
    config: Arc<ProgramConfig>,
    start_attempts: u32,
    command: Command,
    process_name: String,
    kill_command_received: bool,
    process_generation: u32,
    stderr_file: Arc<OutputFile>,
    stdout_file: Arc<OutputFile>,
}

#[derive(Error, Debug, Clone)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub enum RoutineSpawnError {
    #[error("Failed to open stdout file: {0}")]
    OpeningStdoutFile(String),
    #[error("Failed to open stderr file: {0}")]
    OpeningStderrFile(String),
}

impl Routine {
    pub fn spawn(
        config: Arc<ProgramConfig>,
        status_sender: UnboundedSender<NominativeStatus>,
        log_sender: LogSender,
        process_name: String,
        process_generation: u32,
    ) -> Handle {
        let (kill_command_sender, kill_command_receiver) = mpsc::channel(1);
        let command = command::create_command(&config);

        let join_handle = tokio::spawn(async move {
            Self {
                stdout_file: Arc::clone(config.stdout()),
                stderr_file: Arc::clone(config.stderr()),
                config,
                log_sender,
                status_sender: StatusSender::new(status_sender, process_name.clone()),
                kill_command_receiver,
                start_attempts: 0,
                command,
                process_name,
                kill_command_received: false,
                process_generation,
            }
            .routine()
            .await
        });
        Handle::new(join_handle, kill_command_sender)
    }

    async fn routine(mut self) {
        loop {
            let status = self.run_program().await;

            let should_try_restart = self.should_try_restart(&status);

            self.status_sender.send_new_status_to_task_manager(status);
            if self.kill_command_received || !should_try_restart {
                self.status_sender
                    .send_new_status_to_task_manager(Status::NotRestarting {
                        process_generation: self.process_generation,
                    });
                break;
            }
        }
    }

    async fn run_program(&mut self) -> Status {
        let child = {
            // Save the current umask and restore it after the child process is spawned.
            // We need to do this because the child process inherits the umask of the parent process.
            // The mutex is used to ensure that only one thread can save and restore the umask at a time,
            // preventing race conditions.
            //
            // Race condition scenario:
            // Orginal umask: 022
            // Task 1: Saves current umask (022, this is right)
            // Task 1: Sets umask to the one from the config (077, for this example)
            // Task 2: Saves current umask (077, saved the value of the process task 1 is starting since this is the current one, this is not right!)
            // Task 2: Sets umask to the one from the config (007, for this example)
            // Task 1: Starts the subprocess with 007 instead of 077 since the umask was changed by task 2!
            // Task 1: restore the original umask (022, this is right)
            // Task 2: Starts the subprocess with 022 instead of 007 since the umask was restored by task 1!
            // Task 2: restore the original, but incorrect umask (077) because of the previous race condition!
            // Meaning taskmaster ends up with the incorrect umask and the subprocesses were started with the wrong umask.
            static UMASK_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
            let _lock = UMASK_MUTEX.lock().await;

            let taskmaster_umask: mode_t = unsafe { umask(*self.config.umask()) };
            let child = self.child_spawn().await;

            unsafe { umask(taskmaster_umask) };

            child
        };

        match child {
            Ok(child) => {
                self.status_sender
                    .send_new_status_to_task_manager(Status::Starting);
                self.handle_running_child(child).await
            }
            Err(err) => Status::FailedToStartProcess(err.to_string()),
        }
    }

    async fn handle_running_child(&mut self, mut child: Child) -> Status {
        let outputs = Outputs::new(&mut child);
        let listen_task = tokio::spawn(Self::listen(
            outputs,
            Arc::clone(&self.stdout_file),
            Arc::clone(&self.stderr_file),
            self.log_sender.clone(),
            self.process_name.clone(),
        ));

        let status = tokio::select! {
            status = Self::wait_for_child(
                &mut child,
                *self.config.start_time(),
                &mut self.status_sender,
            ) => {
                status
            }

            _ = self.kill_command_receiver.recv() => {
                self.kill_command_received = true;
                Self::kill_subprocess(
                    &mut child,
                    self.config.stop_signal()
                );
                Status::Exited(child.wait().await.expect("error waiting for child"))
            }
        };

        listen_task
            .await
            .expect("error while listening task's output");
        status
    }

    fn kill_subprocess(child: &mut Child, stop_signal: &Signal) {
        if let Some(pid) = child.id() {
            unsafe { kill(pid as i32, *stop_signal as i32) };
        }
    }

    async fn wait_for_child(
        child: &mut Child,
        start_time: u32,
        status_sender: &mut StatusSender,
    ) -> Status {
        if start_time != 0 {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(start_time as u64)) => {}

                // Wait for process to terminate or crash before start_time
                exit_status = child.wait() => {
                    return Status::ErrorDuringStartup(
                        exit_status.expect("Failed to get exit status"),
                    );
                }
            }
        }

        status_sender.send_new_status_to_task_manager(Status::Running);

        // Wait for process to terminate or crash
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
    fn should_try_restart(&mut self, status: &Status) -> bool {
        match status {
            Status::ErrorDuringStartup(_) => self.start_attempts < *self.config.start_retries(),

            _ => {
                self.start_attempts = 0;

                match *self.config.auto_restart() {
                    AutoRestart::False => false,
                    AutoRestart::OnFailure => !self.is_expected_status(status),
                    AutoRestart::True => true,
                }
            }
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
        stdout_file: Arc<OutputFile>,
        stderr_file: Arc<OutputFile>,
        log_sender: LogSender,
        process_name: String,
    ) {
        let stdout = outputs.stdout;
        let stderr = outputs.stderr;

        tokio::join!(
            listen_and_log(
                stdout,
                log_sender.clone(),
                stdout_file,
                LogType::Stdout,
                &process_name
            ),
            listen_and_log(
                stderr,
                log_sender,
                stderr_file,
                LogType::Stderr,
                &process_name
            ),
        );
    }
}

async fn listen_and_log<R: AsyncBufRead + Unpin>(
    mut output: R,
    mut sender: LogSender,
    output_file: Arc<OutputFile>,
    log_type: LogType,
    name: &str,
) {
    loop {
        let mut buffer = Vec::new();
        let bytes_read = output.read_until(b'\n', &mut buffer).await;

        match bytes_read {
            Ok(0) => break,
            Ok(_) => {
                let log = Log::new(log_type, &buffer, name);
                dispatch_log(log, &mut sender, Arc::clone(&output_file)).await;
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
async fn dispatch_log(log: Log, log_sender: &mut LogSender, output: Arc<OutputFile>) {
    //TODO: move this to task_manager
    match (&*output, &log.log_type) {
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
    let process_name = log.process_name.clone();
    log_sender
        .send(log)
        .inspect_err(|_| {
            eprintln!(
                "Taskmaster error: {}: Log receiver was dropped",
                process_name
            )
        })
        .unwrap()
}
