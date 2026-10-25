use tokio::sync::oneshot;

use crate::daemon::{
    CommandSender, ReloadType, TaskManagerCommand, TasksManagerStatus,
    tasks_manager::ServerCommandError,
};

type Result<T, Err = ServerCommandError> = core::result::Result<T, Err>;

#[allow(dead_code)] //TODO: Remove that
pub struct Handle {
    command_sender: CommandSender,
}

impl Handle {
    pub fn new(command_sender: CommandSender) -> Handle {
        Handle { command_sender }
    }

    pub async fn status(&self) -> Result<TasksManagerStatus> {
        let (sender, receiver) = oneshot::channel();

        self.send(TaskManagerCommand::ListProcesses(sender)).await?;

        receiver
            .await
            .map_err(|_| ServerCommandError::ShutdownInProgress)
    }

    pub async fn reload(&self, reload_type: ReloadType) -> Result<()> {
        self.send(TaskManagerCommand::Reload(reload_type)).await
    }

    pub async fn start(&self, task_name: String) -> Result<()> {
        self.send(TaskManagerCommand::StartProgram { task_name })
            .await
    }

    pub async fn restart(&self, task_name: String) -> Result<()> {
        self.send(TaskManagerCommand::RestartProgram { task_name })
            .await
    }

    pub async fn stop(&self, task_name: String) -> Result<()> {
        self.send(TaskManagerCommand::StopProgram { task_name })
            .await
    }

    pub async fn stop_all_processes(&self) -> Result<()> {
        self.send(TaskManagerCommand::StopAllProcesses).await
    }

    pub async fn restore_config_file(&self) -> Result<()> {
        self.send(TaskManagerCommand::RestoreConfigFile).await
    }

    pub async fn shutdown(&self) -> Result<()> {
        match self.send(TaskManagerCommand::Shutdown).await {
            Ok(()) => Ok(()),
            Err(ServerCommandError::ShutdownInProgress) => Ok(()),
            Err(err) => Err(err),
        }
    }

    async fn send(&self, command: TaskManagerCommand) -> Result<()> {
        let (sender, receiver) = oneshot::channel();

        self.command_sender
            .send((command, sender))
            // If the receiver is dropped, that means the routine is shutting down / already shut
            // down
            .map_err(|_| ServerCommandError::ShutdownInProgress)?;

        receiver
            .await
            // If the sender is dropped, that means the routine had a shutdown command in the
            // command queue before our command
            .map_err(|_| ServerCommandError::ShutdownInProgress)?
    }
}
