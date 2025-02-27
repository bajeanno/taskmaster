use std::{mem::ManuallyDrop, sync::Arc};

use commands::{ClientCommand, ServerCommand};
use connection::Connection;
use tokio::{io::DuplexStream, sync::Mutex, task::JoinHandle};

use crate::{client_handler::ClientHandler, task_manager::MockTaskManagerTrait};

type TestConnection = Connection<DuplexStream, ClientCommand, ServerCommand>;

pub struct TestServer {
    join_handle: Option<JoinHandle<()>>,
}

impl TestServer {
    fn new(server: DuplexStream, task_manager: MockTaskManagerTrait) -> Self {
        let task_manager = Arc::new(Mutex::new(task_manager));

        Self {
            join_handle: Some(tokio::spawn(async move {
                ClientHandler::process_client(server, task_manager)
                    .await
                    .unwrap();
            })),
        }
    }

    pub async fn check_errors(mut self, client: TestConnection) {
        drop(client);
        self.join_handle.take().unwrap().await.unwrap();
        let _ = ManuallyDrop::new(self);
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        panic!("TestServer::check_errors() was not called");
    }
}

pub async fn setup_test(task_manager: MockTaskManagerTrait) -> (TestConnection, TestServer) {
    let (client, server) = tokio::io::duplex(4096);

    let mut client = TestConnection::new(client, 4096);

    let server = TestServer::new(server, task_manager);

    let frame = client.read_frame().await.unwrap();
    assert_eq!(frame, Some(ClientCommand::SuccessfulConnection));

    (client, server)
}
