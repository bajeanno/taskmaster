mod config;
mod config_state;
mod error;
mod output_file;
mod process_handler;
mod tasks_manager;

use crate::config_state::ConfigState;
use config::ProgramConfig;
use error::Error;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use tasks_manager::TaskManagerCommand;

const DEFAULT_PORT: i32 = 4444;
const PID_FILE: &str = "/var/run/taskmaster.pid";

type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
struct Args {
    port: i32,
}

fn main() {
    let _ = entrypoint().inspect_err(|err| eprintln!("{err}"));
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
    let pids = get_taskmaster_pids()?;
    if pids.is_empty() {
        let buf = std::process::id().to_string();
        File::create(PID_FILE)?.write_all(buf.as_bytes())?;
    } else {
        todo!("taskmaster is already running: need to write exit routine for this case");
    }
    Ok(())
}

fn get_taskmaster_pids() -> Result<Vec<u64>> {
    let mut buf = Vec::new();
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(PID_FILE)
        .map_err(Error::FailedToOpenPidFile)?
        .read_to_end(&mut buf)?;
    file
        .to_string()
        .split("\n")
        .map(|line| Ok(line.parse()?))
        .collect::<Result<Vec<u64>>>()
}

#[allow(dead_code)]
// TODO: use this function on exit
fn erase_taskmaster_pids() -> Result<()> {
    OpenOptions::new()
        .create(true)
        .truncate(true)
        .open("/var/run/taskmaster.pid")
        .map_err(Error::FailedToOpenPidFile)?;
    Ok(())
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
        .block_on(async { Result::<()>::Ok(()) })
}
