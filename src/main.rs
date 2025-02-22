mod client_handler;

use std::{fmt::Display, io};

use tokio::net::{TcpListener, ToSocketAddrs};

use client_handler::ClientHandler;

#[derive(Debug)]
struct Task {
    id: u32,
    name: String,
}

impl Task {
    fn new(task_id: u32, name: &str) -> Self {
        Self {
            id: task_id,
            name: String::from(name),
        }
    }
}

struct TaskServer {
    listener: TcpListener,
    tasks: Vec<Task>,
}

impl TaskServer {
    async fn new(addr: impl ToSocketAddrs) -> Result<Self, io::Error> {
        Ok(Self {
            listener: TcpListener::bind(addr).await?,
            tasks: Vec::new(),
        })
    }

    async fn run(mut self) {
        self.create_task(""); // TODO delete me

        loop {
            let (socket, _) = self.listener.accept().await.unwrap();
            tokio::spawn(async move { ClientHandler::process_client(socket).await });
        }
    }

    fn create_task(&mut self, task_name: &str) {
        self.tasks
            .push(Task::new(self.tasks.len() as u32, task_name));
    }
}

impl Display for TaskServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tasks: Vec<String> = self
            .tasks
            .iter()
            .map(|task| format!("{}\t{}", task.id, task.name))
            .collect();
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

    unsafe {
        daemonize::Daemonize::new()
            .stdout("./server_output")
            .stderr("./server_output")
            .start()
            .expect("Failed to daemonize server")
    }

    tokio::runtime::Runtime::new()
        .expect("Failed to init tokio runtime")
        .block_on(async {
            TaskServer::new(format!("127.0.0.1:{port}"))
                .await
                .expect("Failed to init TaskServer")
                .run()
                .await;
        });
}
