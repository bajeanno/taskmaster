mod config;
mod error;
mod process_handler;
mod tasks_manager;

use std::sync::Arc;

use config::{Config, Program};
use error::Error;

const DEFAULT_PORT: i32 = 4444;

use tasks_manager::{ServerCommandError, TaskManagerCommand};
use tokio::sync::{mpsc, oneshot};

pub type CommandReceiver =
    mpsc::UnboundedReceiver<(TaskManagerCommand, oneshot::Sender<ServerCommandError>)>;
pub type CommandSender =
    mpsc::UnboundedSender<(TaskManagerCommand, oneshot::Sender<ServerCommandError>)>;

#[allow(dead_code)]
#[derive(Debug)]
pub struct NominativeStatus {
    pub process_name: String,
    pub status: crate::process_handler::Status,
}

#[derive(Debug)]
struct Args {
    port: i32,
}

fn main() {
    let _ = entrypoint().inspect_err(|err| eprintln!("{err}"));
}

fn entrypoint() -> Result<(), Error> {
    let Args { port } = parse_args(std::env::args().nth(1))?;

    let tasks = get_tasks_from_config("taskmaster.yaml");
    let tasks = convert_tasks_to_arc(tasks);

    if !cfg!(debug_assertions) {
        daemonize()?
    }

    start_server(port, tasks)
}

fn convert_tasks_to_arc(programs: Vec<Program>) -> Vec<Arc<Program>> {
    programs.into_iter().map(Arc::new).collect()
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

fn get_tasks_from_config(config_file: &str) -> Vec<Program> {
    match Config::parse(config_file) {
        Ok(config) => config.programs,
        Err(err) => {
            eprintln!("Warning {err}");
            Vec::new()
        }
    }
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

fn start_server(_port: i32, _tasks: Vec<Arc<Program>>) -> Result<(), Error> {
    tokio::runtime::Runtime::new()
        .expect("Failed to init tokio runtime")
        .block_on(async { Result::<(), Error>::Ok(()) })
}
