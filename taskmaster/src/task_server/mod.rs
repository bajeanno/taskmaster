mod error;
pub use error::Error;
use error::Result;

use std::sync::Arc;

use tokio::{
    net::{TcpListener, ToSocketAddrs},
    sync::Mutex,
};

use crate::{Program, client_handler::ClientHandler, task_manager::TaskManager};

pub struct TaskServer {
    task_manager: Arc<Mutex<TaskManager>>,
    listener: TcpListener,
}

impl TaskServer {
    pub async fn new(tasks: Vec<Program>, addr: impl ToSocketAddrs + Into<String>) -> Result<Self> {
        Ok(Self {
            task_manager: Arc::new(Mutex::new(TaskManager::new(tasks))),
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
            let task_manager = Arc::clone(&self.task_manager);
            tokio::spawn(async move {
                ClientHandler::process_client(socket, task_manager)
                    .await
                    .inspect_err(|err| eprintln!("ClientHandler error: {err:?}"))
            });
        }
    }
}
