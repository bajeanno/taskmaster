use std::sync::Arc;

use crate::{
    config::program::ProgramDiff,
    config_state::ConfigState::{self, Active, LoadError, Uninitialized},
    tasks_manager::{ServerCommandError, routine::Routine},
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
                            self.processes
                                .stop_and_remove_program(name)
                                .await
                                .expect("program should be in the processes map");
                            self.processes
                                .start_program(
                                    new_program_config,
                                    &self.status_sender,
                                    &self.log_sender,
                                )
                                .await;
                        }
                        ProgramDiff::NumProcsChanged { before, after } => {
                            self.processes
                                .handle_num_procs_diff(
                                    new_program_config,
                                    before,
                                    after,
                                    name,
                                    &self.status_sender,
                                    &self.log_sender,
                                )
                                .await;
                        }
                        ProgramDiff::Other => {
                            self.processes
                                .update_processes_program_data(name, new_program_config)
                                .await;
                        }
                    }
                }

                None => {
                    self.processes
                        .start_program(new_program_config, &self.status_sender, &self.log_sender)
                        .await;
                }
            }
        }

        for name in current_config.programs.keys() {
            if !new_config.programs.contains_key(name) {
                self.processes
                    .stop_and_remove_program(name)
                    .await
                    .expect("program should be in the processes map");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::str::FromStr;
    use std::sync::Arc;

    use signal::Signal;

    use crate::config::{AutoRestart, Command};
    use crate::config_state::ConfigState;
    use crate::process_handler::{LogReceiver, Status, StatusReceiver};
    use crate::tasks_manager::process_registry::ProcessRegistry;
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
            processes: ProcessRegistry::new(),
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
            .as_inner()
            .lock()
            .await
            .get("unchanged")
            .unwrap()
            .iter()
            .map(|process| process.instance_id())
            .collect();
        let changed_ids_before: Vec<u64> = routine
            .processes
            .as_inner()
            .lock()
            .await
            .get("changed_increased")
            .unwrap()
            .iter()
            .map(|process| process.instance_id())
            .collect();

        routine
            .processes
            .stop_program("changed_decreased")
            .await
            .unwrap();
        routine
            .processes
            .stop_program("changed_increased")
            .await
            .unwrap();

        routine.update_processes(&current_config, &new_config).await;
        {
            let process_registry = routine.processes.as_inner();
            let processes = process_registry.lock().await;

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

        routine.processes.stop_and_join_all_processes().await;
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
            .as_inner()
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

        let process_registry = routine.processes.as_inner();
        let procs_lock = process_registry.lock().await;
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

        routine.processes.stop_and_join_all_processes().await;
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
            .as_inner()
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

        let process_registry = routine.processes.as_inner();
        let procs_lock = process_registry.lock().await;
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

        routine.processes.stop_and_join_all_processes().await;
    }

    #[tokio::test]
    async fn reload_updates_config_on_non_destructive_changes() {
        let current_yaml = r#"programs:
  app:
    cmd: "sleep 30"
    numprocs: 1
    autostart: true
    exitcodes: [0]
    startretries: 1
    stoptime: 2
    stopsignal: "SIGTERM"
    autorestart: false
    clearenv: false"#;

        let new_yaml = r#"programs:
  app:
    cmd: "sleep 30"
    numprocs: 1
    autostart: true
    exitcodes: [0, 2]
    startretries: 5
    stoptime: 10
    stopsignal: "SIGINT"
    autorestart: true
    clearenv: true"#;

        let current_config = config(current_yaml);
        let new_config = config(new_yaml);

        let (mut routine, _status_receiver, _log_receiver) = test_routine(&current_config).await;

        let before_id = routine
            .processes
            .as_inner()
            .lock()
            .await
            .get("app")
            .unwrap()
            .first()
            .unwrap()
            .instance_id();
        assert!(
            routine
                .processes
                .as_inner()
                .lock()
                .await
                .get("app")
                .unwrap()
                .first()
                .unwrap()
                .is_running()
        );

        routine.update_processes(&current_config, &new_config).await;

        {
            let process_registry = routine.processes.as_inner();
            let processes = process_registry.lock().await;
            let process = processes.get("app").unwrap().first().unwrap();
            assert_eq!(
                process.instance_id(),
                before_id,
                "process must not be restarted on a non-destructive reload"
            );
            assert!(
                process.is_running(),
                "process must keep running on a non-destructive reload"
            );

            let updated_config = process.program_config();
            assert_eq!(*updated_config.exit_codes(), vec![0, 2]);
            assert_eq!(*updated_config.start_retries(), 5);
            assert_eq!(*updated_config.stop_time(), 10);
            assert_eq!(updated_config.stop_signal(), &Signal::SIGINT);
            assert_eq!(*updated_config.auto_restart(), AutoRestart::True);
            assert!(updated_config.clear_env());
        }

        routine.processes.stop_and_join_all_processes().await;
    }

    #[tokio::test]
    async fn reload_restarts_program_on_destructive_changes() {
        let current_yaml = r#"programs:
  app:
    cmd: "sleep 30"
    autostart: true"#;

        let new_yaml = r#"programs:
  app:
    cmd: "sleep 31"
    autostart: true"#;

        let current_config = config(current_yaml);
        let new_config = config(new_yaml);

        let (mut routine, _status_receiver, _log_receiver) = test_routine(&current_config).await;

        let before_id = routine
            .processes
            .as_inner()
            .lock()
            .await
            .get("app")
            .unwrap()
            .first()
            .unwrap()
            .instance_id();

        routine.update_processes(&current_config, &new_config).await;

        {
            let process_registry = routine.processes.as_inner();
            let processes = process_registry.lock().await;
            let process = processes.get("app").unwrap().first().unwrap();
            assert_ne!(
                process.instance_id(),
                before_id,
                "process must be restarted on a destructive reload"
            );
            assert!(
                process.is_running(),
                "restarted process must be running after reload"
            );
            assert_eq!(
                process.program_config().cmd(),
                &Command::from_str("sleep 31").unwrap(),
                "process must run with the reloaded command"
            );
        }

        routine.processes.stop_and_join_all_processes().await;
    }

    #[tokio::test]
    async fn reload_updates_config_of_stopped_process_without_starting_it() {
        let current_yaml = r#"programs:
  app:
    cmd: "sleep 30"
    numprocs: 1
    autostart: true
    startretries: 1"#;

        let new_yaml = r#"programs:
  app:
    cmd: "sleep 30"
    numprocs: 1
    autostart: true
    autostart-on-reload: false
    startretries: 5"#;

        let current_config = config(current_yaml);
        let new_config = config(new_yaml);

        let (mut routine, _status_receiver, _log_receiver) = test_routine(&current_config).await;

        routine.processes.stop_program("app").await.unwrap();
        assert!(
            !routine
                .processes
                .as_inner()
                .lock()
                .await
                .get("app")
                .unwrap()
                .first()
                .unwrap()
                .is_running()
        );

        routine.update_processes(&current_config, &new_config).await;

        {
            let process_registry = routine.processes.as_inner();
            let processes = process_registry.lock().await;
            let process = processes.get("app").unwrap().first().unwrap();
            assert_eq!(
                *process.program_config().start_retries(),
                5,
                "stopped process must still receive the reloaded config"
            );
            assert!(
                !process.is_running(),
                "stopped process must not be started by a non-destructive reload"
            );
        }

        routine.processes.stop_and_join_all_processes().await;
    }

    #[tokio::test]
    async fn reload_non_destructive_change_is_propagated_to_running_subroutine() {
        let current_yaml = r#"programs:
  app:
    cmd: "sh -c 'sleep 0.1; exit 1'"
    numprocs: 1
    autostart: true
    autorestart: unexpected
    exitcodes: [1]"#;

        let new_yaml = r#"programs:
  app:
    cmd: "sh -c 'sleep 0.1; exit 1'"
    numprocs: 1
    autostart: true
    autorestart: unexpected
    exitcodes: [0]"#;

        let current_config = config(current_yaml);
        let new_config = config(new_yaml);

        let (mut routine, mut status_receiver, _log_receiver) = test_routine(&current_config).await;

        assert!(
            matches!(
                status_receiver.recv().await.unwrap().status,
                Status::Starting
            ),
            "expected the initial program to start"
        );
        assert!(
            matches!(
                status_receiver.recv().await.unwrap().status,
                Status::Running
            ),
            "expected the initial program to be running"
        );

        routine.update_processes(&current_config, &new_config).await;

        // The reloaded config makes exit code 1 unexpected, so once the
        // program exits the subroutine must restart it instead of giving up.
        // Skip any statuses (e.g. the extra Running emitted when the config is
        // received) until the program exits.
        let mut status = status_receiver.recv().await.unwrap().status;
        while !matches!(status, Status::Exited(_)) {
            status = status_receiver.recv().await.unwrap().status;
        }

        assert!(
            matches!(
                status_receiver.recv().await.unwrap().status,
                Status::Starting
            ),
            "subroutine must restart the process using the reloaded config"
        );
        assert!(
            matches!(
                status_receiver.recv().await.unwrap().status,
                Status::Running
            ),
            "restarted process must be running"
        );

        routine.processes.stop_and_join_all_processes().await;
    }
}
