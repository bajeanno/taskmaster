mod client_handler;
mod error;
mod parser;
mod server;
mod tasks_manager;

use error::{Error, Result};
use parser::program::{Config, Program};
use server::Server;

struct Args {
    port: i32,
}

fn main() {
    let _ = entrypoint().inspect_err(|err| eprintln!("{err}"));
}

fn entrypoint() -> Result<()> {
    let Args { port } = parse_args()?;

    let tasks = get_tasks_from_config("taskmaster.yaml");

    if !cfg!(debug_assertions) {
        daeemonize()?
    }

    start_server(port, tasks)
}

fn parse_args() -> Result<Args> {
    let Some(port) = std::env::args().nth(1) else {
        return Err(Error::InvalidArguments);
    };
    let port: i32 = port
        .parse()
        .map_err(|error| Error::PortArgumentIsNotAnInteger { input: port, error })?;

    Ok(Args { port })
}

fn get_tasks_from_config(config_file: &str) -> Vec<Program> {
    Config::parse(config_file).unwrap_or_else(|err| {
        eprintln!("Warning: {err}");
        Vec::new()
    })
}

fn daeemonize() -> Result<()> {
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
