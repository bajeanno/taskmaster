use std::fmt::Display;

use tokio::net::{TcpListener, ToSocketAddrs};

use super::{
    client_handler::ClientHandler,
    error::{Error, Result},
};
use crate::Program;

pub struct TaskServer {
    tasks: Vec<Program>,
    listener: TcpListener,
}

impl TaskServer {
    pub async fn new(tasks: Vec<Program>, addr: impl ToSocketAddrs + Into<String>) -> Result<Self> {
        Ok(Self {
            tasks,
            listener: TcpListener::bind(&addr).await.map_err(|error| {
                Error::FailedToBindTcpListener {
                    addr: addr.into(),
                    error,
                }
            })?,
        })
    }

    pub async fn run(self) {
        loop {
            let (socket, _) = self.listener.accept().await.unwrap();
            tokio::spawn(async move {
                ClientHandler::process_client(socket)
                    .await
                    .inspect_err(|err| eprintln!("ClientHandler error: {err:?}"))
            });
        }
    }

    pub fn _list_tasks(&self) -> String {
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
