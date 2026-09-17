mod config;
mod config_state;
mod error;
mod output_file;
mod pid_file;
mod process;
mod process_handler;
mod tasks_manager;

#[cfg(test)]
mod tests;

use crate::{config_state::ConfigState, pid_file::PidFile, tasks_manager::ServerCommandError};
use config::ProgramConfig;
use error::{Error, PidError::OtherInstanceRunning};
use tasks_manager::TaskManagerCommand;
use tokio::sync::{mpsc, oneshot};

const DEFAULT_PORT: i32 = 4444;

pub type CommandReceiver = mpsc::UnboundedReceiver<(
    TaskManagerCommand,
    oneshot::Sender<Result<(), ServerCommandError>>,
)>;
pub type CommandSender = mpsc::UnboundedSender<(
    TaskManagerCommand,
    oneshot::Sender<Result<(), ServerCommandError>>,
)>;

#[derive(Debug)]
struct Args {
    port: i32,
}

#[derive(Debug)]
struct Claim();

impl Claim {
    fn new() -> Result<Self, Error> {
        let mut pid_file = PidFile::open()?;
        match pid_file.read_pid()? {
            Some(_) => Err(OtherInstanceRunning)?,
            None => {
                pid_file.write_pid()?;
                Ok(Claim())
            }
        }
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        PidFile::open().unwrap().truncate();
    }
}

fn main() {
    let _ = entrypoint().inspect_err(|err| eprintln!("{err}"));
}

fn entrypoint() -> Result<(), Error> {
    let _pid_file = Claim::new()?;
    let Args { port } = parse_args(std::env::args().nth(1))?;

    if !cfg!(debug_assertions) {
        daemonize()?
    }

    // TODO: replace None with an Optional arguments that specifies the config
    // file name
    start_server(port)
}

fn parse_args(port: Option<String>) -> Result<Args, Error> {
    let port = port
        .map(|port| {
            port.parse()
                .map_err(|error| Error::PortArgumentIsNotAnInteger { input: port, error })
        })
        .unwrap_or(Ok(DEFAULT_PORT))?;

    Ok(Args { port })
}

fn daemonize() -> Result<(), Error> {
    unsafe {
        daemonize::Daemonize::new()
            .stdout("./server_output")
            .stderr("./server_output")
            .start()?
    }
    Ok(())
}

fn start_server(_port: i32) -> Result<(), Error> {
    let _config_manager = ConfigState::from_default_config_file();

    tokio::runtime::Runtime::new()
        .expect("Failed to init tokio runtime")
        .block_on(async { Result::<(), Error>::Ok(()) })
}
