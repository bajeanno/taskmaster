mod client_handler;
mod parser;

use parser::{program::Program, Parser};
use std::{fmt::Display, io};

use tokio::net::{TcpListener, ToSocketAddrs};

use client_handler::ClientHandler;

struct TaskServer {
    tasks: Vec<Program>,
    listener: TcpListener,
}

impl TaskServer {
    async fn new(tasks: Vec<Program>, addr: impl ToSocketAddrs) -> Result<Self, io::Error> {
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

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 2 {
        panic!("Usage: {} <port:i32>\nPort is missing", args[0]);
    }
    let port: i32 = args[1]
        .parse()
        .unwrap_or_else(|err| panic!("Usage: {} <port:i32>\nFailed to parse port: {err}", args[0]));

    let tasks: Vec<Program> = Parser::parse("taskmaster.yaml").unwrap_or_else(|err| {
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
}
