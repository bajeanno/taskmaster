mod reload;

use crate::{
    config::ProgramConfig,
    config_state::ConfigState::Active,
    tasks_manager::{
        ServerCommandError, TaskManagerCommand,
        routine::{Client, Routine},
    },
};
use std::sync::Arc;

impl Routine {
    pub async fn handle_command(
        &mut self,
        command: TaskManagerCommand,
    ) -> Result<(), ServerCommandError> {
        match command {
            TaskManagerCommand::ListProcesses(list_sender) => {
                list_sender
                    .send(self.pool.list_processes().await)
                    .expect("Receiver should never be dropped");
            }

            TaskManagerCommand::Reload { config_file_name } => {
                self.reload_config(&config_file_name).await?;
            }

            TaskManagerCommand::StartProgram { program_name } => {
                self.handle_start_program_command(program_name).await?
            }

            TaskManagerCommand::RestartProgram { program_name } => {
                self.pool.stop_program(&program_name).await?;
                self.handle_start_program_command(program_name).await?
            }

            TaskManagerCommand::StopProgram { program_name } => {
                self.pool.stop_program(program_name).await?
            }

            TaskManagerCommand::SubscribeToProgramEvents {
                program_name,
                client,
            } => {
                self.handle_subscribe_to_program_events(program_name, client)
                    .await?
            }

            TaskManagerCommand::UnsubscribeToProgramEvents {
                program_name,
                client,
            } => {
                self.handle_unsubscribe_to_program_events(program_name, client)
                    .await?
            }

            TaskManagerCommand::StopAllProcesses => self.pool.stop_and_join_all_processes().await,

            TaskManagerCommand::Exit => {
                panic!("Exit command should be handled by Routine::event_listener")
            }
        }
        Ok(())
    }

    async fn handle_start_program_command(
        &mut self,
        program_name: String,
    ) -> Result<(), ServerCommandError> {
        let program_config = self
            .get_program_config(program_name.as_str())
            .ok_or(super::ServerCommandError::NoSuchProgram(program_name))?;
        self.pool
            .start_program(&program_config, &self.status_sender, &self.log_sender)
            .await;
        Ok(())
    }

    async fn handle_subscribe_to_program_events(
        &mut self,
        program_name: String,
        client: Client,
    ) -> Result<(), ServerCommandError> {
        self.clients
            .lock()
            .await
            .get(&program_name)
            .ok_or(super::ServerCommandError::NoSuchProgram(program_name))?
            .add(client);
        Ok(())
    }

    async fn handle_unsubscribe_to_program_events(
        &mut self,
        program_name: String,
        client: Client,
    ) -> Result<(), ServerCommandError> {
        self.clients
            .lock()
            .await
            .get(&program_name)
            .ok_or(super::ServerCommandError::NoSuchProgram(program_name))?
            .remove(client);
        Ok(())
    }

    fn get_program_config(&self, program_name: &str) -> Option<Arc<ProgramConfig>> {
        if let Active(config) = &self.config_state {
            config.programs.get(program_name).map(Arc::clone)
        } else {
            None
        }
    }
}
