mod reload;

use crate::{
    config::ProgramConfig,
    config_state::ConfigState::Active,
    process_handler::NominativeStatus,
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
                    .send(self.list_processes().await)
                    .expect("Receiver should never be dropped");
            }

            TaskManagerCommand::Reload { config_file_name } => {
                self.reload_config(&config_file_name).await?;
            }

            TaskManagerCommand::StartProgram { program_name } => {
                self.handle_start_program_command(program_name).await?
            }

            TaskManagerCommand::RestartProgram { program_name } => {
                self.stop_program(&program_name).await?;
                self.handle_start_program_command(program_name).await?
            }

            TaskManagerCommand::StopProgram { program_name } => {
                self.stop_program(program_name).await?
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

            TaskManagerCommand::StopAllProcesses => self.stop_and_join_all_processes().await,

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
        self.start_program(&program_config).await;
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

    async fn list_processes(&self) -> Vec<Vec<NominativeStatus>> {
        self.processes
            .lock()
            .await
            .iter()
            .map(|(name, processes)| {
                processes
                    .iter()
                    .enumerate()
                    .map(|(id, process)| NominativeStatus {
                        process_name: format!("{}-{}", name, id),
                        status: process.nominative_status.status.clone(),
                    })
                    .collect()
            })
            .collect()
    }

    async fn stop_program(
        &mut self,
        program_name: impl AsRef<str> + Into<String>,
    ) -> Result<(), ServerCommandError> {
        let mut processes = self.processes.lock().await;

        for process in processes
            .get_mut(program_name.as_ref())
            .ok_or_else(|| ServerCommandError::NoSuchProgram(program_name.into()))?
            .iter_mut()
        {
            process.stop_and_join_if_running().await;
        }
        Ok(())
    }

    fn get_program_config(&self, program_name: &str) -> Option<Arc<ProgramConfig>> {
        if let Active(config) = &self.config_state {
            config.programs.get(program_name).map(Arc::clone)
        } else {
            None
        }
    }

    async fn stop_and_remove_program(
        &mut self,
        program_name: &str,
    ) -> Result<(), ServerCommandError> {
        let mut processes = self.processes.lock().await;

        for process in processes
            .get_mut(program_name)
            .ok_or_else(|| ServerCommandError::NoSuchProgram(program_name.into()))?
            .iter_mut()
        {
            process.stop_and_join_if_running().await;
        }
        processes.remove(program_name);
        Ok(())
    }
}
