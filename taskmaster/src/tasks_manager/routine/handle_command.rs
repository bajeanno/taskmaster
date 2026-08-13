use crate::{
    config::ProgramConfig,
    config_state::ConfigState::{self, Active, LoadError, Uninitialized},
    process_handler::NominativeStatus,
    tasks_manager::{
        ServerCommandError, TaskManagerCommand,
        routine::{Client, ProgramDiff, Routine},
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

    async fn reload_config(&mut self, file: &str) -> Result<(), ServerCommandError> {
        match ConfigState::from_config(Some(file)) {
            Active(new_config) => {
                let current_config_state = self.config_state.take();
                self.config_state = Active(new_config.clone());
                match current_config_state {
                    Active(current_config) => {
                        self.update_processes(&current_config, &new_config).await
                    }
                    Uninitialized | LoadError { error: _ } => {
                        self.start_programs(&new_config.programs).await
                    }
                }
            }

            ConfigState::LoadError { error } => {
                return Err(ServerCommandError::FailedToLoadNewConfig(error));
            }

            ConfigState::Uninitialized => {
                panic!("Programmatic error: config_state is Uninitialized after reload")
            }
        }
        Ok(())
    }

    // Even though we don't lock anything between the moment we stop the program
    // and the moment we remove it from the map, it is still impossible for a
    // second user to start the program after we stopped it and before we
    // remove it from the hashmap because we only handle one user command
    // at a time
    async fn update_processes(
        &mut self,
        current_config: &Arc<crate::config::Config>,
        new_config: &Arc<crate::config::Config>,
    ) {
        for (name, new_program) in new_config.programs.iter() {
            match current_config.programs.get(name) {
                Some(current_program) => match Self::program_diff(current_program, new_program) {
                    ProgramDiff::NeedRestart => {
                        self.stop_and_remove_program(name)
                            .await
                            .expect("program should be in the processes map");
                        self.start_program(new_program).await;
                    }
                    ProgramDiff::NumProcsChanged { before, after } => {
                        self.handle_num_procs_diff(new_program, before, after, name)
                            .await;
                    }
                    ProgramDiff::Other => {}
                },

                None => {
                    self.start_program(new_program).await;
                }
            }
        }

        for name in current_config.programs.keys() {
            if !new_config.programs.contains_key(name) {
                self.stop_and_remove_program(name)
                    .await
                    .expect("program should be in the processes map");
            }
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use crate::config_state::ConfigState;
    use crate::process_handler::{LogReceiver, StatusReceiver};
    use crate::tasks_manager::routine::Routine;
    use tokio::sync::{Mutex, mpsc};

    const CURRENT_CONFIG: &str = r#"programs:
    unchanged:
        cmd: "sleep 30"
        autostart: true
    changed_increased:
        cmd: "sleep 30"
        numprocs: 1
        autostart: true
    changed_decreased:
        cmd: "sleep 30"
        numprocs: 2
        autostart: true
    changed_autostart:
        cmd: "sleep 30"
        numprocs: 1
        autostart: false
    removed:
        cmd: "sleep 30"
        autostart: true"#;

    const NEW_CONFIG: &str = r#"programs:
    unchanged:
        cmd: "sleep 30"
        autostart: true
    changed_increased:
        cmd: "sleep 30"
        numprocs: 2
        autostart: true
        autostart-on-reload: true
    changed_decreased:
        cmd: "sleep 30"
        numprocs: 1
        autostart: true
        autostart-on-reload: true
    changed_autostart:
        cmd: "sleep 30"
        numprocs: 2
        autostart: true
    added:
        cmd: "sleep 30"
        autostart: true"#;

    fn config(content: &str) -> Arc<crate::config::Config> {
        match ConfigState::from_content(content.to_string()) {
            ConfigState::Active(config) => config,
            ConfigState::Uninitialized => panic!("config should be active"),
            ConfigState::LoadError { error } => panic!("config should parse: {error}"),
        }
    }

    async fn test_routine(
        current_config: &Arc<crate::config::Config>,
    ) -> (Routine, StatusReceiver, LogReceiver) {
        let (status_sender, status_receiver) = mpsc::unbounded_channel();
        let (log_sender, log_receiver) = mpsc::unbounded_channel();
        let (_command_sender, command_receiver) = mpsc::unbounded_channel();

        let mut routine = Routine {
            config_state: ConfigState::Active(Arc::clone(current_config)),
            processes: Arc::new(Mutex::new(HashMap::new())),
            clients: Arc::new(Mutex::new(HashMap::new())),
            command_receiver,
            log_sender,
            status_sender,
        };
        routine.start_programs(&current_config.programs).await;

        (routine, status_receiver, log_receiver)
    }

    #[tokio::test]
    async fn reload_keeps_unchanged_programs_running() {
        let current_config = config(CURRENT_CONFIG);
        let new_config = config(NEW_CONFIG);

        let (mut routine, _status_receiver, _log_receiver) = test_routine(&current_config).await;

        let unchanged_ids_before: Vec<u32> = routine
            .processes
            .lock()
            .await
            .get("unchanged")
            .unwrap()
            .iter()
            .map(|process| process.instance_id())
            .collect();
        let changed_ids_before: Vec<u32> = routine
            .processes
            .lock()
            .await
            .get("changed_increased")
            .unwrap()
            .iter()
            .map(|process| process.instance_id())
            .collect();

        routine.stop_program("changed_decreased").await.unwrap();
        routine.stop_program("changed_increased").await.unwrap();
        routine.stop_program("changed_autostart").await.unwrap();

        routine.update_processes(&current_config, &new_config).await;
        {
            let processes = routine.processes.lock().await;

            let unchanged = processes
                .get("unchanged")
                .expect("unchanged program should stay registered");
            assert_eq!(
                unchanged
                    .iter()
                    .map(|process| process.instance_id())
                    .collect::<Vec<_>>(),
                unchanged_ids_before,
                "unchanged program must not be restarted on reload"
            );
            assert!(
                unchanged.iter().all(|process| process.is_running()),
                "unchanged program must keep running on reload"
            );

            let changed = processes
                .get("changed_increased")
                .expect("changed program should be re-registered");
            assert_ne!(
                changed
                    .iter()
                    .map(|process| process.instance_id())
                    .collect::<Vec<_>>(),
                changed_ids_before,
                "changed program must be restarted on reload"
            );

            let changed = processes
                .get("changed_decreased")
                .expect("changed program should be re-registered");
            assert_ne!(
                changed
                    .iter()
                    .map(|process| process.instance_id())
                    .collect::<Vec<_>>(),
                changed_ids_before,
                "changed program must be restarted on reload"
            );
            assert!(
                !processes
                    .get("changed_autostart")
                    .unwrap()
                    .iter()
                    .any(|p| p.is_running()),
                "changed_autostart program must not be running on reload"
            );
            assert!(
                processes
                    .get("changed_increased")
                    .unwrap()
                    .iter()
                    .all(|p| p.is_running()),
                "changed_increased program must be running on reload"
            );
            assert!(
                processes
                    .get("changed_decreased")
                    .unwrap()
                    .iter()
                    .all(|p| p.is_running()),
                "changed_decreased program must be running on reload"
            );

            assert!(
                processes.contains_key("added"),
                "new program must be started on reload"
            );
            assert!(
                !processes.contains_key("removed"),
                "removed program must be stopped on reload"
            );
        }

        routine.stop_and_join_all_processes().await;
    }

    #[tokio::test]
    async fn reload_numprocs_increase_preserves_existing_processes() {
        let current_yaml = r#"programs:
    scale:
        cmd: "sleep 30"
        numprocs: 2
        autostart: true
        autostart-on-reload: true"#;

        let increase_yaml = r#"programs:
    scale:
        cmd: "sleep 30"
        numprocs: 3
        autostart: true
        autostart-on-reload: true"#;

        let current_config = config(current_yaml);
        let increase_config = config(increase_yaml);

        let (mut routine, _status_receiver, _log_receiver) = test_routine(&current_config).await;

        // initial state: 2 procs
        let before_ids: Vec<u32> = routine
            .processes
            .lock()
            .await
            .get("scale")
            .unwrap()
            .iter()
            .map(|p| p.instance_id())
            .collect();
        assert_eq!(before_ids.len(), 2);

        // increase to 3: existing indices 0 and 1 should not be restarted
        routine
            .update_processes(&current_config, &increase_config)
            .await;
        let procs_lock = routine.processes.lock().await;
        let scale_procs = procs_lock.get("scale").expect("scale should exist");
        assert_eq!(scale_procs.len(), 3);
        assert_eq!(
            scale_procs[0].instance_id(),
            before_ids[0],
            "existing proc index 0 must not be restarted on scale up"
        );
        assert_eq!(
            scale_procs[1].instance_id(),
            before_ids[1],
            "existing proc index 1 must not be restarted on scale up"
        );
        assert!(
            scale_procs.iter().all(|p| p.is_running()),
            "all procs should be running after scale up"
        );
        drop(procs_lock);

        routine.stop_and_join_all_processes().await;
    }

    #[tokio::test]
    async fn reload_numprocs_decrease_preserves_remaining_processes() {
        let current_yaml = r#"programs:
    scale:
        cmd: "sleep 30"
        numprocs: 2
        autostart: true"#;

        let decrease_yaml = r#"programs:
    scale:
        cmd: "sleep 30"
        numprocs: 1
        autostart: true"#;

        let current_config = config(current_yaml);
        let decrease_config = config(decrease_yaml);

        let (mut routine, _status_receiver, _log_receiver) = test_routine(&current_config).await;

        // initial state: 2 procs
        let before_ids: Vec<u32> = routine
            .processes
            .lock()
            .await
            .get("scale")
            .unwrap()
            .iter()
            .map(|p| p.instance_id())
            .collect();
        assert_eq!(before_ids.len(), 2);
        // decrease to 1: remaining index 0 should not be restarted
        routine
            .update_processes(&current_config, &decrease_config)
            .await;
        let procs_lock = routine.processes.lock().await;
        let scale_procs = procs_lock
            .get("scale")
            .expect("scale should exist after decrease");
        assert_eq!(scale_procs.len(), 1);
        assert_eq!(
            scale_procs[0].instance_id(),
            before_ids[0],
            "index 0 must still be the original instance after scale down"
        );
        assert!(
            scale_procs.iter().all(|p| p.is_running()),
            "remaining proc should be running after scale down"
        );
        drop(procs_lock);

        routine.stop_and_join_all_processes().await;
    }
}
