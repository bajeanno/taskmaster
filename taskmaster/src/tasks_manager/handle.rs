use tokio::{sync::oneshot, task::JoinHandle as TokioJoinHandle};

use crate::{
    TaskManagerCommand,
    tasks_manager::{CommandSender, ServerCommandError},
};

type JoinHandle = TokioJoinHandle<()>;

#[allow(dead_code)] //TODO: Remove that
pub struct Handle {
    command_sender: CommandSender,
    join_handle: JoinHandle,
}

#[allow(dead_code)] //TODO: Remove that
impl Handle {
    pub fn new(command_sender: CommandSender, join_handle: JoinHandle) -> Handle {
        Handle {
            command_sender,
            join_handle,
        }
    }

    pub async fn send(&self, command: TaskManagerCommand) -> Result<(), ServerCommandError> {
        let (sender, receiver) = oneshot::channel();
        self.command_sender
            .send((command, sender))
            .expect("Receiver should never be dropped");
        receiver
            .await
            .expect("error while waiting for response from sub-routine")
    }

    pub async fn stop(self) {
        self.send(TaskManagerCommand::Exit)
            .await
            .expect("Exit command failed");
        self.join_handle.await.expect("error awaiting task manager");
    }
}
