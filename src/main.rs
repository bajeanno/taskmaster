mod client_handler;
mod error;
mod parser;

use error::{Error, Result};

use parser::program::{Config, Program};

use std::{fmt::Display, io};

use tokio::net::{TcpListener, ToSocketAddrs};

use client_handler::ClientHandler;

struct TaskServer {
    tasks: Vec<Program>,
    listener: TcpListener,
}

impl TaskServer {
    async fn new(tasks: Vec<Program>, addr: impl ToSocketAddrs) -> core::result::Result<Self, io::Error> {
        Ok(Self {
            tasks,
            listener: TcpListener::bind(addr).await?,
        })
    }

    async fn run(self) {
        println!("{}", self.list_tasks()); // TODO: remove

        loop {
            let (socket, _) = self.listener.accept().await.unwrap();
            tokio::spawn(async move { ClientHandler::process_client(socket).await });
        }
    }

    fn list_tasks(&self) -> String {
        println!("{:<15}{:^50}{:10}", "program name", "cmd", "pids");
        self.tasks
            .iter()
            .fold(String::new(), |acc, value| format!("{acc}{}\n", value))
    }
}

impl Display for TaskServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tasks: Vec<String> = self.tasks.iter().map(|value| format!("{value}")).collect();
        write!(f, "{}", tasks.join("\n"))
    }
}

fn entrypoint() -> Result<()> {
    let Some(port) = std::env::args().nth(1) else {
        return Err(Error::InvalidArguments);
    };
    let port: i32 = port
        .parse()
        .map_err(|error| Error::PortArgumentIsNotAnInteger { input: port, error })?;

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
            TaskServer::new(tasks, format!("127.0.0.1:{port}"))
                .await
                .expect("Failed to init TaskServer")
                .run()
                .await;
        });

    Ok(())
}

fn main() {
    let _ = entrypoint().inspect_err(|err| eprintln!("{err}"));
}
