mod handle_command;

use super::Process;
use super::TaskManagerCommand;
use super::handle::Handle;
use crate::CommandReceiver;
use crate::config_state::ConfigState;
use crate::process_handler::NominativeStatus;
use crate::tasks_manager::ServerCommandError;
use crate::{
    config::ProgramConfig,
    process_handler::{LogReceiver, LogSender, Status, StatusReceiver},
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{
    Mutex,
    mpsc::{self, UnboundedSender},
};

// Mocking Client struct brought by the rpc-genie crate
pub struct Client {}

impl Client {
    fn send(&self) {}
}

type ClientMap = Arc<Mutex<HashMap<String, SubscribedClients>>>;

struct SubscribedClients;

impl SubscribedClients {
    fn add(&self, _client: Client) {}
    fn remove(&self, _client: Client) {}
    fn for_each(&self, _callback: impl FnMut(&Client)) {}
}

#[allow(dead_code)] //TODO: Remove that
pub struct Routine {
    config_state: ConfigState,
    clients: ClientMap,
    processes: Arc<Mutex<HashMap<String, Vec<Process>>>>,
    command_receiver: CommandReceiver,
    log_sender: LogSender,
    status_sender: UnboundedSender<NominativeStatus>,
}

#[allow(dead_code)] //TODO: remove that
impl Routine {
    pub fn spawn(config_state: ConfigState) -> Handle {
        let (log_sender, log_receiver) = mpsc::unbounded_channel();
        let (status_sender, status_receiver) = mpsc::unbounded_channel();
        let (command_sender, command_receiver) = mpsc::unbounded_channel();

        let handle = tokio::spawn(async move {
            Self {
                config_state,
                processes: Arc::new(Mutex::new(HashMap::new())),
                clients: Arc::new(Mutex::new(HashMap::new())),
                command_receiver,
                log_sender,
                status_sender,
            }
            .routine(status_receiver, log_receiver)
            .await;
        });

        Handle::new(command_sender, handle)
    }

    async fn routine(mut self, status_receiver: StatusReceiver, log_receiver: LogReceiver) {
        if let ConfigState::Active(config) = &self.config_state {
            self.start_programs(&Arc::clone(config).programs).await;
        }

        let logs_handle = tokio::spawn(Self::listen_for_logs(log_receiver, self.clients.clone()));
        let status_handle = tokio::spawn(Self::listen_for_status(
            status_receiver,
            Arc::clone(&self.processes),
        ));
        self.event_listener().await;

        logs_handle.abort();
        status_handle.abort();
    }

    async fn start_programs(&mut self, programs: &HashMap<String, Arc<ProgramConfig>>) {
        for (_, program_config) in programs.iter() {
            self.create_program_processes(program_config).await;
        }
    }

    async fn create_program_processes(&mut self, program_config: &Arc<ProgramConfig>) {
        let num_procs: usize = *program_config.num_procs() as usize;
        let vec: Vec<Process> = (0..num_procs)
            .map(|id| {
                Process::new(program_config.clone(), id)
                    .auto_start(self.status_sender.clone(), self.log_sender.clone())
            })
            .collect();
        self.processes
            .lock()
            .await
            .insert(program_config.name().clone(), vec);
    }

    fn split_process_name(mut process_name: String) -> Option<(String, usize)> {
        let dash_index = process_name.rfind('-')?;
        let tmp = process_name.split_off(dash_index);
        let id: usize = tmp[1..].parse().ok()?;
        let program_name = process_name;
        Some((program_name, id))
    }

    fn get_program_name(process_name: &str) -> Option<String> {
        if let Some((program_name, _)) = Self::split_process_name(process_name.to_string()) {
            Some(program_name)
        } else {
            None
        }
    }

    async fn listen_for_status(
        mut status_receiver: StatusReceiver,
        processes: Arc<Mutex<HashMap<String, Vec<Process>>>>,
    ) {
        while let Some(nominative_status) = status_receiver.recv().await {
            let mut processes = processes.lock().await;
            let (program_name, id) =
                Self::split_process_name(nominative_status.process_name.clone())
                    .expect("Error: process name does not contain process id");
            if let Some(processes) = processes.get_mut(&program_name) {
                if let Some(process) = processes.get_mut(id) {
                    if let Status::NotRestarting { instance_id } = nominative_status.status
                        && process.instance_id() == instance_id
                    {
                        process.join_if_running().await;
                    }
                    process.nominative_status = nominative_status;
                }
            }
        }
    }

    /// logs are already written to log files, we only need to write them to the client if he asks for it
    async fn listen_for_logs(mut log_receiver: LogReceiver, clients: ClientMap) {
        while let Some(log) = log_receiver.recv().await {
            if let Some(clients) = clients
                .lock()
                .await
                .get(&Self::get_program_name(log.process_name.as_str()).unwrap_or(log.process_name))
            {
                clients.for_each(Client::send);
            }
        }
    }

    async fn event_listener(&mut self) {
        while let Some((command, sender)) = self.command_receiver.recv().await {
            if matches!(command, TaskManagerCommand::Exit) {
                self.stop_and_join_all_processes().await;
                sender
                    .send(Ok(()))
                    .expect("Receiver should never be dropped");
                break;
            }

            sender
                .send(self.handle_command(command).await)
                .expect("Receiver should never be dropped");
        }
    }

    async fn stop_and_join_all_processes(&mut self) {
        let mut processes = self.processes.lock().await;

        for process in processes
            .iter_mut()
            .flat_map(|(_, process_vec)| process_vec.iter_mut())
        {
            process.stop_and_join_if_running().await;
        }
    }

    async fn compare_programs(
        &mut self,
        current_program: Arc<ProgramConfig>,
        new_program: Arc<ProgramConfig>,
    ) {
        if current_program.cmd == new_program.cmd {
            todo!("restart after reload")
        }
        let (current_num_procs, new_num_procs) = (
            *current_program.num_procs() as usize,
            *new_program.num_procs() as usize,
        );
        self.handle_num_procs_diff(
            &current_program,
            current_num_procs,
            new_num_procs,
            current_program.name(),
        )
        .await;
    }

    async fn handle_num_procs_diff(
        &mut self,
        program_config: &Arc<ProgramConfig>,
        current_num_procs: usize,
        new_num_procs: usize,
        program_name: &str,
    ) {
        let procs_delta = current_num_procs as isize - new_num_procs as isize;
        if procs_delta < 0 {
            for process in &mut self.processes.lock().await.get_mut(program_name).unwrap()
                [current_num_procs..new_num_procs]
            {
                process.stop_and_join_if_running().await;
            }
        }

        if procs_delta > 0 {
            //TODO: check if process if currently running
            //TODO: if so, create process + .auto_start()
            //TODO: else, only create process.
            let mut lock = self.processes.lock().await;
            let Some(process_vec) = lock.get_mut(program_name) else {
                panic!("program is uninitialized");
            };
            for id in current_num_procs..=new_num_procs {
                process_vec.insert(id, Process::new(Arc::clone(program_config), id));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tasks_manager::routine::Routine;

    #[test]
    fn test_split_process_name() {
        let process_name = "taskmaster_test_task-0".to_string();
        assert_eq!(
            Routine::split_process_name(process_name),
            Some(("taskmaster_test_task".to_string(), 0))
        );
    }
}
