mod client_handler;
mod error;
mod parser;
mod task_manager;
mod task_server;

use error::{Error, Result};
use parser::program::{Config, Program};
use task_server::TaskServer;

fn entrypoint() -> Result<()> {
    let port = std::env::args()
        .nth(1)
        .map(|port| {
            port.parse()
                .map_err(|error| Error::PortArgumentIsNotAnInteger { input: port, error })
        })
        .unwrap_or(Ok(4444))?;

    let tasks: Vec<Program> = Config::parse("taskmaster.yaml").unwrap_or_else(|err| {
        eprintln!("Warning: {err}");
        Vec::new()
    });

    if !cfg!(debug_assertions) {
        unsafe {
            daemonize::Daemonize::new()
                .stdout("./server_output")
                .stderr("./server_output")
                .start()
                .expect("Failed to daemonize server")
        }
    }

    tokio::runtime::Runtime::new()
        .expect("Failed to init tokio runtime")
        .block_on(async {
            TaskServer::new(tasks, format!("localhost:{port}"))
                .await?
                .run()
                .await;
            Result::<()>::Ok(())
        })
}

fn main() {
    let _ = entrypoint().inspect_err(|err| eprintln!("{err}"));
}
