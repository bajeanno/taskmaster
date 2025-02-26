use commands::ClientCommand;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::task_server::{
    client_handler::{ClientHandler, Result},
    task_manager::TaskManagerTrait,
};

impl<Stream, TaskManager> ClientHandler<Stream, TaskManager>
where
    Stream: AsyncWrite + AsyncRead + Unpin,
    TaskManager: TaskManagerTrait,
{
    pub(in super::super) async fn handle_list_tasks(&mut self) -> Result<()> {
        eprintln!("Client {} requested ListTasks", self.client_id);

        let task_list = self.task_manager.lock().await.list_tasks();
        let response = ClientCommand::TaskList(task_list);

        eprintln!("Client {} ListTasks response: {response:?}", self.client_id);
        self.write_frame(&response).await
    }
}

#[cfg(test)]
mod test {
    use commands::{ClientCommand, ServerCommand};

    use crate::task_server::{client_handler, task_manager::MockTaskManagerTrait};

    #[tokio::test]
    async fn test_handle_list_tasks() {
        let mut mock_task_manager = MockTaskManagerTrait::new();
        mock_task_manager
            .expect_list_tasks()
            .once()
            .return_once(|| "List of tasks".to_string());

        let (mut client, server) = client_handler::test_utils::setup_test(mock_task_manager).await;

        client
            .write_frame(&ServerCommand::ListTasks)
            .await
            .unwrap();
        let frame = client.read_frame().await.unwrap();
        assert_eq!(
            frame,
            Some(ClientCommand::TaskList("List of tasks".to_string()))
        );

        server.check_errors(client).await;
    }
}
