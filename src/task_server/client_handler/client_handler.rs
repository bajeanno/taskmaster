use std::sync::Mutex;

use super::error::{Error, Result};
use commands::{ClientCommands, ServerCommands};
use connection::Connection;
use tokio::io::{AsyncRead, AsyncWrite};

pub struct ClientHandler<Stream> {
    client_id: u64,
    connection: Connection<Stream, ServerCommands, ClientCommands>,
}

impl<Stream> ClientHandler<Stream>
where
    Stream: AsyncWrite + AsyncRead + Unpin,
{
    pub async fn process_client(socket: Stream) -> Result<()> {
        let mut handler = Self::new(socket)?;

        handler
            .write_frame(&ClientCommands::SuccessfulConnection)
            .await?;

        handler.handle_loop().await
    }

    fn new(socket: Stream) -> Result<Self> {
        static NEXT_CLIENT_ID: Mutex<u64> = Mutex::new(0);

        let mut next_client_id = NEXT_CLIENT_ID
            .lock()
            .expect("ClientHandler::new() mutex is poisoned");

        let handler = Self {
            client_id: *next_client_id,
            connection: Connection::new(socket, 4096),
        };
        *next_client_id += 1;

        Ok(handler)
    }

    async fn handle_loop(mut self) -> Result<()> {
        while let Some(command) = self.read_frame().await? {
            match command {
                ServerCommands::Test => self.write_frame(&ClientCommands::Test).await?,
            };
        }
        Ok(())
    }

    async fn read_frame(&mut self) -> Result<Option<ServerCommands>> {
        match self.connection.read_frame().await {
            Ok(value) => Ok(value),
            Err(error) => {
                let _ = self.write_frame(&ClientCommands::FailedToParseFrame).await;
                Err(Error::FailedToReadFrameFromClient {
                    client_id: self.client_id,
                    error,
                })
            }
        }
    }

    async fn write_frame(&mut self, frame: &ClientCommands) -> Result<()> {
        match self.connection.write_frame(frame).await {
            Ok(value) => Ok(value),
            Err(error) => Err(Error::FailedToWriteFrameFromClient {
                client_id: self.client_id,
                error,
            }),
        }
    }
}
