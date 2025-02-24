use commands::{ClientCommands, ServerCommands};
use connection::Connection;
use tokio::io::{AsyncRead, AsyncWrite};

pub struct ClientHandler<Stream> {
    connection: Connection<Stream, ServerCommands, ClientCommands>,
}

impl<Stream> ClientHandler<Stream>
where
    Stream: AsyncWrite + AsyncRead + Unpin,
{
    pub async fn process_client(socket: Stream) {
        let mut handler = Self {
            connection: Connection::new(socket, 4096),
        };

        handler
            .write_frame(&ClientCommands::SuccessfulConnection)
            .await;

        handler.handle_loop().await;
    }

    async fn handle_loop(mut self) {
        while let Some(command) = self.read_frame().await {
            match command {
                ServerCommands::Test => self.write_frame(&ClientCommands::Test).await,
            };
        }
    }

    async fn read_frame(&mut self) -> Option<ServerCommands> {
        match self.connection.read_frame().await {
            Ok(frame) => frame,
            Err(err) => {
                let _ = self
                    .connection
                    .write_frame(&ClientCommands::FailedToParseFrame)
                    .await;
                panic!("Failed to read frame from client: {err}");
            }
        }
    }

    async fn write_frame(&mut self, frame: &ClientCommands) {
        self.connection
            .write_frame(frame)
            .await
            .expect("Failed to write frame to client")
    }
}
