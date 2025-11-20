use super::{Handle, Status};
use crate::parser::program::Program;
use std::process::Stdio;
use tokio::{
    io::{AsyncBufReadExt, BufReader, Error},
    process::{ChildStderr, ChildStdout, Command},
    select,
    sync::mpsc,
};

pub type Receiver = mpsc::Receiver<Status>;

#[allow(dead_code)] //TODO: Remove that
pub struct Routine {
    command: Command,
    sender: mpsc::Sender<Status>,
    attach_sender: Option<mpsc::Sender<String>>,
    config: Program,
    start_attempts: u32,
    stdout: Option<ChildStdout>,
    stderr: Option<ChildStderr>,
}

#[allow(dead_code)] //TODO: Remove that
impl Routine {
    pub fn spawn(command: Command, config: Program) -> Result<Handle, Error> {
        let (sender, receiver) = mpsc::channel(100);

        let join_handle = tokio::spawn(async move {
            Self {
                command,
                config,
                sender,
                attach_sender: None,
                start_attempts: 0,
                stdout: None,
                stderr: None,
            }
            .routine()
            .await;
        });
        Ok(Handle::new(join_handle, receiver))
    }

    async fn routine(mut self) {
        while self.start_attempts <= *self.config.start_retries() {
            match self.start().await {
                Ok(()) => {
                    break;
                }
                Err(_) => {
                    todo!();
                }
            }
        }
        self.listen().await;
    }

    async fn start(&mut self) -> Result<(), Error> {
        self.start_attempts += 1;
        // self.sender.send(Status::Running).await.unwrap(); //TODO: verifier si c'est bon (mettre expect)
        let child = self
            .command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(); //TODO: enlever ça
        self.stdout = Some(child.stdout.expect("Failed to open stdout")); //TODO: check that
        self.stderr = Some(child.stderr.expect("Failed to open stderr")); //TODO: check that
        Ok(())
    }

    async fn listen(&mut self) {
        if let Some(stdout) = self.stdout.take()
            && let Some(stderr) = self.stderr.take()
        {
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
            }
        }
    }
}
