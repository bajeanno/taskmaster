#[cfg(test)]
mod test_utils;

mod commands_impl;

mod error;

pub use error::Error;
use error::Result;

use crate::tasks_manager;
use commands::{ClientCommand, ServerCommand};
use connection::Connection;
use std::os::fd::RawFd;
use tokio::io::{AsyncRead, AsyncWrite};

#[derive(Debug, Clone, Copy)]
pub struct ClientId(RawFd);

impl From<RawFd> for ClientId {
    fn from(fd: RawFd) -> Self {
        Self(fd)
    }
}

impl std::fmt::Display for ClientId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct ClientHandler<Stream, TaskManager> {
    client_id: ClientId,
    task_manager: TaskManager,
    connection: Connection<Stream, ServerCommand, ClientCommand>,
}

impl<Stream, TaskManager> ClientHandler<Stream, TaskManager>
where
    Stream: AsyncWrite + AsyncRead + Unpin,
    TaskManager: tasks_manager::Api,
{
    pub async fn process_client(
        socket: Stream,
        task_manager: TaskManager,
        client_id: ClientId,
    ) -> Result<()> {
        let mut handler = Self::new(socket, task_manager, client_id)?;

        handler
            .write_frame(&ClientCommand::SuccessfulConnection)
            .await?;

        handler.event_loop().await
    }

    fn new(socket: Stream, task_manager: TaskManager, client_id: ClientId) -> Result<Self> {
        let handler = Self {
            client_id,
            task_manager,
            connection: Connection::new(socket, 4096),
        };

        eprintln!("Client {} has connected", handler.client_id);
        Ok(handler)
    }

    async fn event_loop(mut self) -> Result<()> {
        while let Some(command) = self.read_frame().await? {
            match command {
                ServerCommand::ListTasks => self.handle_list_tasks(command).await?,
            }
        }
        Ok(())
    }

    async fn read_frame(&mut self) -> Result<Option<ServerCommand>> {
        match self.connection.read_frame().await {
            Ok(value) => Ok(value),
            Err(error) => {
                let _ = self.write_frame(&ClientCommand::FailedToParseFrame).await;
                Err(Error::ReadFrame {
                    client_id: self.client_id,
                    error,
                })
            }
        }
    }

    async fn write_frame(&mut self, frame: &ClientCommand) -> Result<()> {
        match self.connection.write_frame(frame).await {
            Ok(value) => Ok(value),
            Err(error) => Err(Error::WriteFrame {
                client_id: self.client_id,
                error,
            }),
        }
    }
}

impl<Stream, TaskManager> Drop for ClientHandler<Stream, TaskManager> {
    fn drop(&mut self) {
        eprintln!("Client {} has disconnected", self.client_id);
    }
}
