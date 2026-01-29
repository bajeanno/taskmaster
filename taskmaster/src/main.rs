mod client_handler;
mod config;
mod error;
mod process_handler;
mod server;
mod tasks_manager;

use config::{Config, Program};
use error::{Error, Result};
use server::Server;

const DEFAULT_PORT: i32 = 4444;

#[derive(Debug)]
struct Args {
    port: i32,
}

fn main() {
    let _ = entrypoint().inspect_err(|err| eprintln!("{err}"));
}

fn entrypoint() -> Result<()> {
    let Args { port } = parse_args(std::env::args().nth(1))?;

    let tasks = get_tasks_from_config("taskmaster.yaml");

    if !cfg!(debug_assertions) {
        daemonize()?
    }

    start_server(port, tasks)
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

fn get_tasks_from_config(config_file: &str) -> Vec<Program> {
    match Config::parse(config_file) {
        Ok(config) => config.programs,
        Err(err) => {
            eprintln!("Warning {err}");
            Vec::new()
        }
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

fn start_server(port: i32, tasks: Vec<Program>) -> Result<()> {
    tokio::runtime::Runtime::new()
        .expect("Failed to init tokio runtime")
        .block_on(async {
            Server::new(tasks, format!("localhost:{port}"))
                .await?
                .run()
                .await;
            Result::<()>::Ok(())
        })
}
