use crate::{
    client_handler::{ClientHandler, ClientId},
    tasks_manager,
};
use commands::{ClientCommand, ServerCommand};
use connection::Connection;
use tokio::{io::DuplexStream, task::JoinHandle};

type TestConnection = Connection<DuplexStream, ClientCommand, ServerCommand>;

pub struct TestServer {
    join_handle: Option<JoinHandle<()>>,
    check_error_called: bool,
}

impl TestServer {
    fn new(server: DuplexStream, task_manager: tasks_manager::MockApi) -> Self {
        Self {
            join_handle: Some(tokio::spawn(async move {
                ClientHandler::process_client(server, task_manager, ClientId::from(0))
                    .await
                    .unwrap();
            })),
            check_error_called: false,
        }
    }

    pub async fn check_errors(mut self, client: TestConnection) {
        drop(client);
        self.join_handle.take().unwrap().await.unwrap();
        self.check_error_called = true;
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if !self.check_error_called {
            panic!("TestServer::check_errors() was not called");
        }
    }
}

pub async fn setup_test(task_manager: tasks_manager::MockApi) -> (TestConnection, TestServer) {
    let (client, server) = tokio::io::duplex(4096);

    let mut client = TestConnection::new(client, 4096);

    let server = TestServer::new(server, task_manager);

    let frame = client.read_frame().await.unwrap();
    assert_eq!(frame, Some(ClientCommand::SuccessfulConnection));

    (client, server)
}
