use std::sync::Arc;

use crate::{
    config::{ProgramConfig, program::ProgramDiff},
    config_state::ConfigState::{self, Active, LoadError, Uninitialized},
    tasks_manager::{ServerCommandError, process::Process, routine::Routine},
};

impl Routine {
    pub async fn reload_config(&mut self, file: &str) -> Result<(), ServerCommandError> {
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
                Ok(())
            }

            ConfigState::LoadError { error } => {
                Err(ServerCommandError::FailedToLoadNewConfig(error))
            }

            ConfigState::Uninitialized => {
                unreachable!("ConfigState::from_config cannot return Uninitialized")
            }
        }
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
        for (name, new_program_config) in new_config.programs.iter() {
            match current_config.programs.get(name) {
                Some(current_program_config) => {
                    match current_program_config.diff(new_program_config) {
                        ProgramDiff::NeedRestart => {
                            self.stop_and_remove_program(name)
                                .await
                                .expect("program should be in the processes map");
                            self.start_program(new_program_config).await;
                        }
                        ProgramDiff::NumProcsChanged { before, after } => {
                            self.handle_num_procs_diff(new_program_config, before, after, name)
                                .await;
                        }
                        ProgramDiff::Other => {
                            self.update_processes_program_data(name, new_program_config)
                                .await;
                        }
                    }
                }

                None => {
                    self.start_program(new_program_config).await;
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

    async fn handle_num_procs_diff(
        &mut self,
        new_program_config: &Arc<ProgramConfig>,
        current_num_procs: usize,
        new_num_procs: usize,
        program_name: &str,
    ) {
        let procs_delta = current_num_procs as isize - new_num_procs as isize;

        if procs_delta > 0 {
            // new_num_procs cannot be 0 as it's checked in the parsing
            let mut processes_hashmap = self.processes.lock().await;
            let process_vec = processes_hashmap.get_mut(program_name).unwrap();
            for process in process_vec.iter_mut().rev().take(procs_delta as usize) {
                process.stop_and_join_if_running().await;
            }
            process_vec.truncate(new_num_procs);

            for process in process_vec.iter_mut() {
                process
                    .update_program_config(Arc::clone(new_program_config))
                    .await;
                process.auto_start_on_reload(&self.status_sender, &self.log_sender);
            }
        } else if procs_delta < 0 {
            let mut lock = self.processes.lock().await;
            let Some(process_vec) = lock.get_mut(program_name) else {
                panic!("program is uninitialized");
            };

            for id in current_num_procs..new_num_procs {
                process_vec.push(Process::new(Arc::clone(new_program_config), id));
            }

            for process in process_vec.iter_mut() {
                process
                    .update_program_config(Arc::clone(new_program_config))
                    .await;
                process.auto_start_on_reload(&self.status_sender, &self.log_sender);
            }
        }
    }

    async fn update_processes_program_data(
        &self,
        program_name: &str,
        new_config: &Arc<ProgramConfig>,
    ) {
        for process in self
            .processes
            .lock()
            .await
            .get_mut(program_name)
            .expect("program is absent from processes")
            .iter_mut()
        {
            process.update_program_config(Arc::clone(new_config)).await;
        }
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
        numprocs: 1
        autostart: false
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

        let unchanged_ids_before: Vec<u64> = routine
            .processes
            .lock()
            .await
            .get("unchanged")
            .unwrap()
            .iter()
            .map(|process| process.instance_id())
            .collect();
        let changed_ids_before: Vec<u64> = routine
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
                "changed_increased program must not be running on reload"
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
        let before_ids: Vec<u64> = routine
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
        let before_ids: Vec<u64> = routine
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
