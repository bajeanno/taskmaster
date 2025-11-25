use super::{Handle, Status};
use crate::parser::program::Program;
use std::process::{ExitCode, Stdio};
#[allow(unused)] //TODO: remove that
use tokio::{
    io::{AsyncBufReadExt, BufReader, Error},
    process::{Child, ChildStderr, ChildStdout, Command},
    select,
    sync::mpsc,
    time::{Duration, Instant, sleep},
};

pub type Receiver = mpsc::Receiver<Status>;
pub type Sender = mpsc::Sender<Status>;

#[allow(dead_code)] //TODO: Remove that
pub struct Routine {
    sender: Sender,
    attach_sender: Option<mpsc::Sender<String>>,
    config: Program,
    start_attempts: u32,
    stdout: Option<ChildStdout>,
    stderr: Option<ChildStderr>,
    child: Option<Child>,
    status: Status,
}

#[allow(dead_code)] //TODO: Remove that
impl Routine {
    pub fn spawn(config: Program) -> Result<Handle, Error> {
        let (sender, receiver) = mpsc::channel(100);

        let join_handle = tokio::spawn(async move {
            Self {
                config,
                sender,
                attach_sender: None,
                start_attempts: 0,
                stdout: None,
                stderr: None,
                child: None,
                status: Status::NotSpawned,
            }
            .routine()
            .await;
        });
        Ok(Handle::new(join_handle, receiver))
    }

    async fn routine(mut self) {
        let mut start_time: Instant;

        //This is a do-while
        'routine_loop: while {
            start_time = Instant::now();
            match self.start().await {
                Ok(mut child) => {
                    self.status(Status::Starting).await;
                    if let Some(stdout) = self.stdout.take()
                        && let Some(stderr) = self.stderr.take()
                    {
                        let handle = tokio::spawn(async move {
                            listen(stdout, stderr).await;
                        });
                        child.wait().await.expect("error waiting for child");
                        self.status(Status::Exited(ExitCode::from(
                            child.id().unwrap_or_default() as u8,
                        )))
                        .await;
                        handle.await.expect("Error awaiting routine end");
                    }
                }
                Err(err) => {
                    self.status(Status::FailedToStart(String::from(err.to_string())))
                        .await;
                    break 'routine_loop;
                }
            };

            self.start_attempts <= *self.config.start_retries()
                && start_time.elapsed().as_secs() < self.config.start_time().clone().into()
        } {}
    }

    async fn status(&mut self, status: Status) {
        self.status = status.clone();
        self.sender.send(status).await.expect("Receiver was dropped");
    }

    async fn start(&mut self) -> Result<Child, Error> {
        self.start_attempts += 1;
        let mut child = self
            .config
            .cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        self.stdout = Some(child.stdout.take().expect("Failed to open stdout"));
        self.stderr = Some(child.stderr.take().expect("Failed to open stderr"));
        Ok(child)
    }
}

async fn listen(stdout: ChildStdout, stderr: ChildStderr) {
    // , stdout_file: Option<String>, stderr_file: Option<String>
    // add that to create log files.
    let mut stdout = BufReader::new(stdout);
    let mut stderr = BufReader::new(stderr);
    loop {
        let mut stdout_output = Vec::new();
        let mut stderr_output = Vec::new();
        select! {
            read_result = stdout.read_until(b'\n', &mut stdout_output) => {
                if let Ok(result) = read_result {
                    if result == 0 {
                        break;
                    }
                    else {
                        let output = String::from_utf8_lossy(&stdout_output);
                        println!("{}", &output);
                    }
                }
                else {
                    panic!("error reading stdout");
                }
            },
            read_result = stderr.read_until(b'\n', &mut stderr_output) => {
                if let Ok(result) = read_result {
                    if result == 0 {
                        break;
                    }
                    else {
                        let output = String::from_utf8_lossy(&stderr_output);
                        println!("{}", &output);
                    }
                }
                else {
                    panic!("error reading stderr");
                }
            },
        }
        println!("read 1 line");
    }
}
