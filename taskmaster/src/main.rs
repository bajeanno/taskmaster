mod config;
mod config_state;
mod error;
mod output_file;
mod process_handler;
mod tasks_manager;

use crate::config_state::ConfigState;
use config::ProgramConfig;
use error::Error;
use std::fs::{File, OpenOptions, remove_file};
use std::io::{Read, Write};
use tasks_manager::TaskManagerCommand;

const DEFAULT_PORT: i32 = 4444;
const PID_FILE: &str = "/var/run/taskmaster.d/taskmaster.pid";

type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
struct Args {
    port: i32,
}

fn main() {
    let _ = entrypoint().inspect_err(|err| eprintln!("{err}")); // TODO: maybe exit with error code
    let _ = erase_taskmaster_pids().inspect_err(|err| eprintln!("{err}")); // TODO: maybe exit with error code
}

fn entrypoint() -> Result<()> {
    check_already_running()?;
    let Args { port } = parse_args(std::env::args().nth(1))?;

    if !cfg!(debug_assertions) {
        daemonize()?
    }

    // TODO: replace None with an Optional arguments that specifies the config
    // file name
    start_server(port, None)
}

fn check_already_running() -> Result<()> {
    let pid = get_taskmaster_pids()?;
    if pid == 0 {
        File::create(PID_FILE)?.write_all(std::process::id().to_string().as_bytes())?;
    } else {
        todo!("taskmaster is already running: need to write exit routine for this case");
    }
    Ok(())
}

fn get_taskmaster_pids() -> Result<u64> {
    let mut buf = Vec::new();
    let file_content = OpenOptions::new()
        .write(true)
        .create(true)
        .read(true)
        .open(PID_FILE)
        .map_err(Error::FailedToOpenPidFile)?
        .read_to_end(&mut buf)?;
    Ok(file_content.to_string().parse::<u64>()?)
}

#[allow(dead_code)]
// TODO: use this function on exit
fn erase_taskmaster_pids() -> Result<()> {
    OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(PID_FILE)
        .map_err(Error::FailedToOpenPidFile)?;
    Ok(())
}

#[test]
fn pid_check_test() {
    check_already_running().unwrap();
    erase_taskmaster_pids().unwrap();
    check_already_running().unwrap();
    remove_file(PID_FILE).unwrap();
}

fn parse_args(port: Option<String>) -> Result<Args> {
    let port = port
        .map(|port| {
            port.parse()
                .map_err(|error| Error::PortArgumentIsNotAnInteger { input: port, error })
        })
        .unwrap_or(Ok(DEFAULT_PORT))?;

    Ok(Args { port })
}

#[cfg(test)]
mod taskmaster {
    use super::*;

    #[test]
    fn test_parse_args() {
        let mut port = Some("4444".to_string());
        assert_eq!(4444, parse_args(port).unwrap().port);
        port = Some("4443".to_string());
        assert_eq!(4443, parse_args(port).unwrap().port);
        port = Some("0".to_string());
        assert_eq!(0, parse_args(port).unwrap().port);
        port = Some("55".to_string());
        assert_eq!(55, parse_args(port).unwrap().port);

        assert_eq!(DEFAULT_PORT, parse_args(None).unwrap().port);

        port = Some("hey".to_string());
        let Err(Error::PortArgumentIsNotAnInteger { input, error: _ }) = parse_args(port) else {
            panic!("Function parse_args did not return an error")
        };
        assert_eq!(input, "hey");
    }
}

fn daemonize() -> Result<()> {
    unsafe {
        daemonize::Daemonize::new()
            .stdout("./server_output")
            .stderr("./server_output")
            .start()?
    }
    Ok(())
}

fn start_server(_port: i32, config_file: Option<String>) -> Result<()> {
    let _config_manager = ConfigState::from_config(config_file.as_deref());

    tokio::runtime::Runtime::new()
        .expect("Failed to init tokio runtime")
        .block_on(async { Ok(()) })
}
