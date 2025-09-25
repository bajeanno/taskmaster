use commands::{ClientCommand, ServerCommand};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::{
    client_handler::{ClientHandler, Error, Result},
    tasks_manager,
};

impl<Stream, TaskManager> ClientHandler<Stream, TaskManager>
where
    Stream: AsyncWrite + AsyncRead + Unpin,
    TaskManager: tasks_manager::Api,
{
    pub(in crate::client_handler) async fn handle_list_tasks(
        &mut self,
        command: ServerCommand,
    ) -> Result<()> {
        eprintln!("Client {} requested ListTasks", self.client_id);

        let task_list =
            self.task_manager
                .list_tasks()
                .await
                .map_err(|error| Error::HandleCommand {
                    client_id: self.client_id,
                    command,
                    error,
                })?;

        let response = ClientCommand::TaskList(task_list);

        eprintln!("Client {} ListTasks response: {response:?}", self.client_id);
        self.write_frame(&response).await
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::tasks_manager;

    use crate::client_handler;
    use commands::ServerCommand;

    #[tokio::test]
    async fn test_handle_list_tasks() {
        let expected: Vec<String> = (0..2).map(|i| format!("Task {i}")).collect();

        let expected_clone = expected.clone();
        let mut mock_task_manager = tasks_manager::MockApi::new();
        mock_task_manager
            .expect_list_tasks()
            .once()
            .return_once(|| Ok(expected_clone));

        let (mut client, server) = client_handler::test_utils::setup_test(mock_task_manager).await;

        client.write_frame(&ServerCommand::ListTasks).await.unwrap();
        let frame = client.read_frame().await.unwrap();
        assert_eq!(frame, Some(ClientCommand::TaskList(expected)));

        server.check_errors(client).await;
    }
}
