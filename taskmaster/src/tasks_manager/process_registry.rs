use std::{collections::HashMap, sync::Arc};

use tokio::sync::{Mutex, mpsc::UnboundedSender};

use crate::{
    config::ProgramConfig,
    process_handler::{LogSender, NominativeStatus, Status},
    tasks_manager::{ServerCommandError, process::Process, split_process_name},
};

// TODO: remove that
#[allow(dead_code)]
pub struct ProcessRegistry(
    Mutex<HashMap<String, Vec<Process>>>,
);

// TODO: remove that
#[allow(dead_code)]
impl ProcessRegistry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self (Mutex::new(HashMap::new())))
    }

    pub async fn start_program(
        &self,
        program_config: &Arc<ProgramConfig>,
        status_sender: &UnboundedSender<NominativeStatus>,
        log_sender: &LogSender,
    ) {
        let mut pool = self.0.lock().await;
        match pool.get_mut(program_config.name()) {
            Some(process_vec) => {
                process_vec
                    .iter_mut()
                    .for_each(|p| p.start(status_sender, log_sender));
            }
            None => {
                pool.insert(
                    program_config.name().clone(),
                    self.create_processes_vec(program_config, status_sender, log_sender)
                        .await,
                );
            }
        }
    }

    async fn create_processes_vec(
        &self,
        program_config: &Arc<ProgramConfig>,
        status_sender: &UnboundedSender<NominativeStatus>,
        log_sender: &LogSender,
    ) -> Vec<Process> {
        (0..*program_config.num_procs() as usize)
            .map(|id| {
                Process::new(program_config.clone(), id).auto_start(status_sender, log_sender)
            })
            .collect()
    }

    pub async fn stop_program(
        &self,
        program_name: impl AsRef<str> + Into<String>,
    ) -> Result<(), ServerCommandError> {
        let mut processes = self.0.lock().await;

        for process in processes
            .get_mut(program_name.as_ref())
            .ok_or_else(|| ServerCommandError::NoSuchProgram(program_name.into()))?
            .iter_mut()
        {
            process.stop_and_join_if_running().await;
        }
        Ok(())
    }

    pub async fn stop_and_remove_program(
        &self,
        program_name: &str,
    ) -> Result<(), ServerCommandError> {
        let mut processes = self.0.lock().await;

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

    pub async fn stop_and_join_all_processes(&self) {
        let mut processes = self.0.lock().await;

        for process in processes
            .iter_mut()
            .flat_map(|(_, process_vec)| process_vec.iter_mut())
        {
            process.stop_and_join_if_running().await;
        }
    }

    pub async fn list_processes(&self) -> Vec<Vec<NominativeStatus>> {
        self.0
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

    pub async fn update_processes_program_data(
        &self,
        program_name: &str,
        new_config: &Arc<ProgramConfig>,
    ) {
        for process in self
            .0
            .lock()
            .await
            .get_mut(program_name)
            .expect("program is absent from processes")
            .iter_mut()
        {
            process.update_program_config(Arc::clone(new_config)).await;
        }
    }

    pub async fn handle_num_procs_diff(
        &self,
        new_program_config: &Arc<ProgramConfig>,
        current_num_procs: usize,
        new_num_procs: usize,
        program_name: &str,
        status_sender: &UnboundedSender<NominativeStatus>,
        log_sender: &LogSender,
    ) {
        let procs_delta = current_num_procs as isize - new_num_procs as isize;

        if procs_delta > 0 {
            // new_num_procs cannot be 0 as it's checked in the parsing
            let mut processes_hashmap = self.0.lock().await;
            let process_vec = processes_hashmap.get_mut(program_name).unwrap();
            for process in process_vec.iter_mut().rev().take(procs_delta as usize) {
                process.stop_and_join_if_running().await;
            }
            process_vec.truncate(new_num_procs);

            for process in process_vec.iter_mut() {
                process
                    .update_program_config(Arc::clone(new_program_config))
                    .await;
                process.auto_start_on_reload(status_sender, log_sender);
            }
        } else if procs_delta < 0 {
            let mut lock = self.0.lock().await;
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
                process.auto_start_on_reload(status_sender, log_sender);
            }
        }
    }

    pub async fn store_status(&self, nominative_status: NominativeStatus) {
        let mut processes = self.0.lock().await;
        let (program_name, id) = split_process_name(nominative_status.process_name.clone())
            .expect("Error: process name does not contain process id");
        if let Some(processes) = processes.get_mut(&program_name)
            && let Some(process) = processes.get_mut(id)
        {
            if let Status::NotRestarting { instance_id } = nominative_status.status
                && process.instance_id() == instance_id
            {
                process.join_if_running().await;
            }
            process.nominative_status = nominative_status;
        }
    }

    #[cfg(test)]
    pub fn as_inner(&self) -> &Mutex<HashMap<String, Vec<Process>>> {
        &self.0
    }
}
