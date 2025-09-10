mod error;
pub use error::Error;
use error::Result;

use crate::{
    Program,
    client_handler::{ClientHandler, ClientId},
    tasks_manager,
};
use std::os::fd::AsRawFd;
use tokio::net::{TcpListener, ToSocketAddrs};

pub struct Server {
    tasks_manager: tasks_manager::Handle,
    listener: TcpListener,
}

impl Server {
    pub async fn new(tasks: Vec<Program>, addr: impl ToSocketAddrs + Into<String>) -> Result<Self> {
        let tasks_manager = tasks_manager::spawn(tasks).await;

        Ok(Self {
            tasks_manager,
            listener: TcpListener::bind(&addr)
                .await
                .map_err(|error| Error::BindTcpListener {
                    addr: addr.into(),
                    error,
                })?,
        })
    }

    pub async fn run(self) {
        loop {
            let (socket, _) = self.listener.accept().await.unwrap();

            let tasks_manager = self.tasks_manager.clone();

            tokio::spawn(async move {
                let client_id = ClientId::from(socket.as_raw_fd());
                ClientHandler::process_client(socket, tasks_manager, client_id)
                    .await
                    .inspect_err(|err| eprintln!("ClientHandler error: {err:?}"))
            });
        }
    }
}
