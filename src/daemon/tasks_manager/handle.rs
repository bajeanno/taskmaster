use std::pin::Pin;

use tokio::{sync::oneshot, task::JoinHandle};

use crate::daemon::{
    CommandSender, ReloadType, TaskManagerCommand, TasksManagerStatus,
    tasks_manager::ServerCommandError,
};

type Result<T, Err = ServerCommandError> = core::result::Result<T, Err>;

#[allow(dead_code)] //TODO: Remove that
pub struct Handle {
    command_sender: CommandSender,
    join_handle: JoinHandle<()>,
}

impl Handle {
    pub fn new(command_sender: CommandSender, join_handle: JoinHandle<()>) -> Handle {
        Handle {
            command_sender,
            join_handle,
        }
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

    pub async fn start(&self, program_name: String) -> Result<()> {
        self.send(TaskManagerCommand::StartProgram { program_name })
            .await
    }

    pub async fn restart(&self, program_name: String) -> Result<()> {
        self.send(TaskManagerCommand::RestartProgram { program_name })
            .await
    }

    pub async fn stop(&self, program_name: String) -> Result<()> {
        self.send(TaskManagerCommand::StopProgram { program_name })
            .await
    }

    pub async fn stop_all_processes(&self) -> Result<()> {
        self.send(TaskManagerCommand::StopAllProcesses).await
    }

    pub async fn restore_config_file(&self) -> Result<()> {
        self.send(TaskManagerCommand::RestoreConfigFile).await
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.send(TaskManagerCommand::Shutdown).await
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

impl Future for Handle {
    type Output = <JoinHandle<()> as Future>::Output;

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        Pin::new(&mut self.join_handle).poll(cx)
    }
}
