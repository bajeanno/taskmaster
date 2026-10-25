mod reload;

use crate::daemon::{
    config::ProgramConfig,
    config_state::ConfigState::Active,
    tasks_manager::{
        ServerCommandError, TaskManagerCommand,
        routine::{Client, Routine},
    },
};
use std::sync::Arc;

pub enum ShouldStopEventLoop {
    True,
    False,
}

impl Routine {
    pub async fn handle_command(
        &mut self,
        command: TaskManagerCommand,
    ) -> Result<ShouldStopEventLoop, ServerCommandError> {
        match command {
            TaskManagerCommand::ListProcesses(list_sender) => {
                list_sender
                    .send(self.processes.list_processes().await)
                    .expect("Receiver should never be dropped");
            }

            TaskManagerCommand::Reload(reload_args) => {
                self.reload_config(reload_args).await?;
            }

            TaskManagerCommand::StartProgram { task_name } => {
                self.handle_start_program_command(task_name).await?
            }

            TaskManagerCommand::RestartProgram { task_name } => {
                self.processes.stop_program(&task_name).await?;
                self.handle_start_program_command(task_name).await?
            }

            TaskManagerCommand::StopProgram { task_name } => {
                self.processes.stop_program(task_name).await?
            }

            TaskManagerCommand::SubscribeToProgramEvents {
                task_name,
                client,
            } => {
                self.handle_subscribe_to_program_events(task_name, client)
                    .await?
            }

            TaskManagerCommand::UnsubscribeToProgramEvents {
                task_name,
                client,
            } => {
                self.handle_unsubscribe_to_program_events(task_name, client)
                    .await?
            }

            TaskManagerCommand::StopAllProcesses => {
                self.processes.stop_and_join_all_processes().await
            }

            TaskManagerCommand::RestoreConfigFile => {
                self.config_state.restore_config_file()?;
            }

            TaskManagerCommand::Shutdown => {
                self.processes.stop_and_join_all_processes().await;
                return Ok(ShouldStopEventLoop::True);
            }
        }
        Ok(ShouldStopEventLoop::False)
    }

    async fn handle_start_program_command(
        &mut self,
        task_name: String,
    ) -> Result<(), ServerCommandError> {
        let program_config = self
            .get_program_config(task_name.as_str())
            .ok_or(super::ServerCommandError::NoSuchProgram(task_name))?;
        self.processes
            .start_program(&program_config, &self.status_sender, &self.log_sender)
            .await;
        Ok(())
    }

    async fn handle_subscribe_to_program_events(
        &mut self,
        task_name: String,
        client: Client,
    ) -> Result<(), ServerCommandError> {
        self.clients
            .lock()
            .await
            .get(&task_name)
            .ok_or(super::ServerCommandError::NoSuchProgram(task_name))?
            .add(client);
        Ok(())
    }

    async fn handle_unsubscribe_to_program_events(
        &mut self,
        task_name: String,
        client: Client,
    ) -> Result<(), ServerCommandError> {
        self.clients
            .lock()
            .await
            .get(&task_name)
            .ok_or(super::ServerCommandError::NoSuchProgram(task_name))?
            .remove(client);
        Ok(())
    }

    fn get_program_config(&self, task_name: &str) -> Option<Arc<ProgramConfig>> {
        if let Active {
            config,
            config_file_path: _,
        } = &self.config_state
        {
            config.programs.get(task_name).map(Arc::clone)
        } else {
            None
        }
    }
}
