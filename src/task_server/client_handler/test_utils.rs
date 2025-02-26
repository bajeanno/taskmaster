use std::sync::Arc;

use commands::{ClientCommands, ServerCommands};
use connection::Connection;
use tokio::{io::DuplexStream, sync::Mutex};

use crate::task_server::{client_handler::ClientHandler, task_manager::MockTaskManagerTrait};

pub async fn setup_test(
    task_manager: MockTaskManagerTrait,
) -> Connection<DuplexStream, ClientCommands, ServerCommands> {
    let task_manager = Arc::new(Mutex::new(task_manager));

    let (client, server) = tokio::io::duplex(4096);

    let mut client = Connection::new(client, 4096);

    tokio::spawn(async move {
        ClientHandler::process_client(server, task_manager)
            .await
            .unwrap();
    });

    let frame = client.read_frame().await.unwrap();
    assert_eq!(frame, Some(ClientCommands::SuccessfulConnection));

    client
}
