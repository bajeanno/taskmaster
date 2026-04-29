use tokio::sync::oneshot;

use crate::{CommandSender, TaskManagerCommand, tasks_manager::ServerCommandError};

#[allow(dead_code)]
pub struct Handle {
    command_sender: CommandSender,
}

#[allow(dead_code)]
impl Handle {
    pub(super) fn new(command_sender: CommandSender) -> Handle {
        Handle { command_sender }
    }

    pub(super) async fn send(&self, command: TaskManagerCommand) -> Result<(), ServerCommandError> {
        let (sender, receiver) = oneshot::channel();
        self.command_sender
            .send((command, sender))
            .expect("Receiver should never be dropped");
        receiver.await.expect("Receiver failed")
    }
}
