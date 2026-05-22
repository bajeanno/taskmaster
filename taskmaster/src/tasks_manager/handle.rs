use tokio::{sync::oneshot, task::JoinHandle as TokioJoinHandle};

use crate::{CommandSender, TaskManagerCommand, tasks_manager::ServerCommandError};

type JoinHandle = TokioJoinHandle<()>;

#[allow(dead_code)] //TODO: Remove that
pub struct Handle {
    command_sender: CommandSender,
    join_handle: JoinHandle,
}

#[allow(dead_code)] //TODO: Remove that
impl Handle {
    pub(super) fn new(command_sender: CommandSender, join_handle: JoinHandle) -> Handle {
        Handle {
            command_sender,
            join_handle,
        }
    }

    pub(super) async fn send(&self, command: TaskManagerCommand) -> Result<(), ServerCommandError> {
        let (sender, receiver) = oneshot::channel();
        self.command_sender
            .send((command, sender))
            .expect("Receiver should never be dropped");
        receiver.await.expect("Je sais pas pq mais ça marche pas")
    }

    pub(super) async fn stop(self) {
        let result = self.send(TaskManagerCommand::Exit).await;
        let join_handle_result = self.join_handle.await;

        result.expect("failed to send exit command to task manager");
        join_handle_result.expect("error awaiting task manager");
    }
}
