mod config;
mod config_state;
mod error;
mod output_file;
mod process_handler;
mod tasks_manager;

#[cfg(test)]
mod tests;

use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::fd::AsRawFd,
};

use crate::{config_state::ConfigState, tasks_manager::ServerCommandError};
use config::ProgramConfig;
use error::{
    Error,
    PidError::{self, OtherInstanceRunning},
};
use libc::sys::file::{LOCK_EX, LOCK_UN, flock};
use tasks_manager::TaskManagerCommand;
use tokio::sync::{mpsc, oneshot};

const DEFAULT_PORT: i32 = 4444;
const PID_FILE: &str = "/var/run/taskmaster.d/taskmaster.pid";

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

fn main() {
    check_already_running(PID_FILE).unwrap();

    let _ = entrypoint().inspect_err(|err| eprintln!("{err}"));

    erase_file(PID_FILE);
}

fn entrypoint() -> Result<(), Error> {
    let Args { port } = parse_args(std::env::args().nth(1))?;

    if !cfg!(debug_assertions) {
        daemonize()?
    }

    // TODO: replace None with an Optional arguments that specifies the config
    // file name
    start_server(port)
}

fn check_already_running(pid_file: &str) -> Result<(), Error> {
    let mut file = acquire_file_lock(pid_file)?;
    let res = if read_pid(&mut file)?.is_some() {
        Err(OtherInstanceRunning)?
    } else {
        file.write_all(std::process::id().to_string().as_bytes())
            .map_err(PidError::WriteFile)?;
        Ok(())
    };
    release_file_lock(file);
    res
}

fn acquire_file_lock(pid_file: &str) -> Result<File, Error> {
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .read(true)
        .truncate(false)
        .open(pid_file)
        .map_err(PidError::OpenFile)?;
    unsafe { flock(file.as_raw_fd(), LOCK_EX) };
    Ok(file)
}

fn release_file_lock(file: File) {
    unsafe { flock(file.as_raw_fd(), LOCK_UN) };
}

fn read_pid(file: &mut File) -> Result<Option<u32>, Error> {
    let mut buf = String::new();
    file.read_to_string(&mut buf).map_err(PidError::ReadFile)?;
    Ok(match buf.len() == 0 {
        true => None,
        false => Some(buf.parse::<u32>().map_err(PidError::Parse)?),
    })
}

fn erase_file(pid_file: &str) {
    let _ = OpenOptions::new().write(true).truncate(true).open(pid_file);
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
