mod config;
mod config_state;
mod error;
mod output_file;
mod process_handler;
mod tasks_manager;

use crate::{config_state::ConfigState, tasks_manager::ServerCommandError};
use config::ProgramConfig;
use error::Error;
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

fn main() {
    let _ = entrypoint().inspect_err(|err| eprintln!("{err}"));
}

fn entrypoint() -> Result<(), Error> {
    let Args { port } = parse_args(std::env::args().nth(1))?;

    if !cfg!(debug_assertions) {
        daemonize()?
    }

    // TODO: replace None with an Optional arguments that specifies the config
    // file name
    start_server(port, None)
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

fn daemonize() -> Result<(), Error> {
    unsafe {
        daemonize::Daemonize::new()
            .stdout("./server_output")
            .stderr("./server_output")
            .start()?
    }
    Ok(())
}

fn start_server(_port: i32, config_file: Option<String>) -> Result<(), Error> {
    let _config_manager = ConfigState::from_config(config_file.as_deref());

    tokio::runtime::Runtime::new()
        .expect("Failed to init tokio runtime")
        .block_on(async { Result::<(), Error>::Ok(()) })
}
