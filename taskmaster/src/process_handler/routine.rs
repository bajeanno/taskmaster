use super::KillCommandReceiver;
use super::LogSender;
use super::ReloadEventReceiver;
use super::{Handle, NominativeStatus, Status, StatusSender, command};
use crate::config::{AutoRestart, ProgramConfig};
use crate::output_file::OutputFile;
use crate::process::ProcessId;
use crate::process_handler::{Log, LogType, Outputs};
use libc::signal::kill;
use libc::unistd::umask;
use signal::Signal;
use std::panic;
use std::process::Stdio;
use std::sync::Arc;
use thiserror::Error;
use tokio::process::Command;
use tokio::sync::mpsc::UnboundedSender;
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, Error},
    process::Child,
    sync::mpsc,
    time::Duration,
};

pub struct Routine {
    status_sender: StatusSender,
    log_sender: LogSender,
    kill_command_receiver: KillCommandReceiver,
    reload_event_receiver: ReloadEventReceiver,
    config: Arc<ProgramConfig>,
    start_attempts: u32,
    command: Command,
    process_id: ProcessId,
    kill_command_received: bool,
    instance_id: usize,
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
        process_id: ProcessId,
        instance_id: usize,
    ) -> Handle {
        let (kill_command_sender, kill_command_receiver) = mpsc::channel(1);
        let (reload_event_sender, reload_event_receiver) = mpsc::channel(1);
        let command = command::create_command(&config);

        let join_handle = tokio::spawn(async move {
            Self {
                config,
                log_sender,
                status_sender: StatusSender::new(status_sender, process_id.clone()),
                kill_command_receiver,
                reload_event_receiver,
                start_attempts: 0,
                command,
                process_id,
                kill_command_received: false,
                instance_id,
            }
            .routine()
            .await
        });
        Handle::new(join_handle, kill_command_sender, reload_event_sender)
    }

    async fn routine(mut self) {
        loop {
            let status = self.run_program().await;

            let should_try_restart = self.should_try_restart(&status);

            self.status_sender.send_new_status_to_task_manager(status);
            if self.kill_command_received || !should_try_restart {
                self.status_sender
                    .send_new_status_to_task_manager(Status::NotRestarting {
                        instance_id: self.instance_id,
                    });
                break;
            }
        }
    }

    async fn run_program(&mut self) -> Status {
        match self.child_spawn() {
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
            Arc::clone(self.config.stdout()),
            Arc::clone(self.config.stderr()),
            self.log_sender.clone(),
            self.process_id.clone(),
        ));

        loop {
            let maybe_status = tokio::select! {
                wait_status = Self::wait_for_child(
                    &mut child,
                    *self.config.start_time(),
                    &mut self.status_sender,
                ) => {
                     Some(wait_status)
                }

                new_config = self.reload_event_receiver.recv() => {
                    self.config = new_config.expect("reload_event_receiver channel closed");
                    None
                }

                _ = self.kill_command_receiver.recv() => {
                    self.kill_command_received = true;
                    Some(Self::kill_subprocess(
                        &mut child,
                        self.config.stop_signal(),
                        self.config.stop_time(),
                    ).await)
                }
            };

            if let Some(status) = maybe_status {
                listen_task
                    .await
                    .expect("error while listening task's output");
                return status;
            }
        }
    }

    async fn kill_subprocess(child: &mut Child, stop_signal: &Signal, stop_time: &u32) -> Status {
        if let Some(pid) = child.id() {
            unsafe { kill(pid as i32, *stop_signal as i32) };
        }

        tokio::select! {
            status = child.wait() => {
                Status::Exited(status.expect("error waiting for child after sending stop signal"))
            }
            _ = tokio::time::sleep(Duration::from_secs(*stop_time as u64)) => {
                // sending SIGKILL signal to force a process to stop
                // SIGKILL cannot be caught by the subprocess and constitutes a way to force processes to stop
                if let Some(pid) = child.id() {
                    unsafe { kill(pid as i32, Signal::SIGKILL as i32) };
                }
                Status::Exited(child.wait().await.expect("error waiting for child after sending SIGTERM"))
            }
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
    fn child_spawn(&mut self) -> Result<Child, Error> {
        self.start_attempts += 1;
        let config_umask = *self.config.umask();
        let workdir = self.config.working_dir().clone();
        let child = unsafe {
            self.command
                .pre_exec(move || {
                    umask(config_umask);
                    std::env::set_current_dir(workdir.as_str())
                })
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?
        };
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
        process_id: ProcessId,
    ) {
        let stdout = outputs.stdout;
        let stderr = outputs.stderr;

        tokio::join!(
            listen_and_log(
                stdout,
                log_sender.clone(),
                stdout_file,
                LogType::Stdout,
                &process_id
            ),
            listen_and_log(
                stderr,
                log_sender,
                stderr_file,
                LogType::Stderr,
                &process_id
            ),
        );
    }
}

async fn listen_and_log<R: AsyncBufRead + Unpin>(
    mut output: R,
    mut sender: LogSender,
    output_file: Arc<OutputFile>,
    log_type: LogType,
    id: &ProcessId,
) {
    loop {
        let mut buffer = Vec::new();
        let bytes_read = output.read_until(b'\n', &mut buffer).await;

        match bytes_read {
            Ok(0) => break,
            Ok(_) => {
                Log::new(log_type, &buffer, id)
                    .dispatch_log(&mut sender, Arc::clone(&output_file))
                    .await;
            }
            Err(err) => {
                eprintln!("Taskmaster error: {id}: Error encountered while reading stderr: {err}");
                break;
            }
        }
    }
}
