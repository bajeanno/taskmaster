use super::Process;
use super::TaskManagerCommand;
use super::handle::Handle;
use crate::CommandReceiver;
use crate::process_handler::NominativeStatus;
use crate::{
    config::ProgramConfig,
    process_handler::{self, LogReceiver, LogSender, Status, StatusReceiver},
};
use std::collections::{HashMap, hash_map};
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
    program_configs: Arc<Vec<Arc<ProgramConfig>>>,
    clients: ClientMap,
    processes: Arc<Mutex<HashMap<String, Vec<Process>>>>,
    command_receiver: CommandReceiver,
    log_sender: LogSender,
    status_sender: UnboundedSender<NominativeStatus>,
}

#[allow(dead_code)] //TODO: remove that
impl Routine {
    pub fn spawn(program_configs: Vec<Arc<ProgramConfig>>) -> Handle {
        let (log_sender, log_receiver) = mpsc::unbounded_channel();
        let (status_sender, status_receiver) = mpsc::unbounded_channel();
        let (command_sender, command_receiver) = mpsc::unbounded_channel();

        let handle = tokio::spawn(async move {
            Self {
                program_configs: program_configs.into(),
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
        self.start_programs().await;

        let logs_handle = tokio::spawn(Self::listen_for_logs(log_receiver, self.clients.clone()));
        let status_handle = tokio::spawn(Self::listen_for_status(
            status_receiver,
            Arc::clone(&self.processes),
        ));
        self.event_listener().await;

        logs_handle.abort();
        status_handle.abort();
    }

    async fn start_programs(&mut self) {
        for program_config in Arc::clone(&self.program_configs).iter() {
            self.start_program(program_config).await;
        }
    }

    async fn start_program(&mut self, program_config: &Arc<ProgramConfig>) {
        let num_procs: u8 = *program_config.num_procs();

        for id in 0..num_procs {
            self.start_process(id as usize, Arc::clone(program_config))
                .await;
        }
    }

    async fn start_process(&mut self, process_id: usize, program_config: Arc<ProgramConfig>) {
        let process_name = format!("{}-{}", program_config.name(), process_id);

        match self.processes.lock().await.entry(process_name.clone()) {
            hash_map::Entry::Occupied(mut entry) => {
                if !entry.get()[process_id].is_async_task_running() {
                    let process_generation =
                        entry.get()[process_id].process_generation().wrapping_add(1);

                    (*entry.get_mut())[process_id] = self
                        .start_process_handler_routine(
                            program_config,
                            process_name,
                            process_generation,
                        )
                        .await;
                }
            }
            hash_map::Entry::Vacant(entry) => {
                let mut processes: Vec<Process> = (0..*program_config.num_procs())
                    .map(|_| Process::default())
                    .collect();
                processes[process_id] = self
                    .start_process_handler_routine(program_config, process_name, 0)
                    .await;
                entry.insert(processes);
            }
        };
    }

    async fn start_process_handler_routine(
        &self,
        program_config: Arc<ProgramConfig>,
        process_name: String,
        process_generation: u32,
    ) -> Process {
        match process_handler::Routine::spawn(
            program_config,
            self.status_sender.clone(),
            self.log_sender.clone(),
            process_name,
            process_generation,
        )
        .await
        {
            Ok(handle) => Process::new(Some(handle), Status::RoutineStarting, process_generation),
            Err(err) => Process::new(None, Status::FailedToSpawnRoutine(err), process_generation),
        }
    }

    fn split_process_name(mut process_name: String) -> Option<(String, usize)> {
        let dash_index = process_name.rfind('-')?;
        let tmp = process_name.split_off(dash_index);
        let id: usize = tmp[1..].parse().ok()?;
        let program_name = process_name;
        Some((program_name, id))
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
                let process = &mut processes[id];
                if let Status::NotRestarting { process_generation } = nominative_status.status
                    && process.process_generation() == process_generation
                {
                    process.join_if_running().await;
                }
                process.status = nominative_status.status;
            }
        }
    }

    /// logs are already written to log files, we only need to write them to the client if he asks for it
    async fn listen_for_logs(mut log_receiver: LogReceiver, clients: ClientMap) {
        while let Some(log) = log_receiver.recv().await {
            let index = log
                .process_name
                .rfind('-')
                .unwrap_or(log.process_name.len());
            if let Some(clients) = clients.lock().await.get(&log.process_name[0..index]) {
                clients.for_each(Client::send);
            }
        }
    }

    async fn event_listener(&mut self) {
        while let Some((command, sender)) = self.command_receiver.recv().await {
            match command {
                TaskManagerCommand::ListProcesses(list_sender) => {
                    let vec = self
                        .processes
                        .lock()
                        .await
                        .iter()
                        .map(|(name, proceses)| {
                            proceses
                                .iter()
                                .enumerate()
                                .map(|(i, process)| NominativeStatus {
                                    process_name: format!("{name}-{i}"),
                                    status: process.status.clone(),
                                })
                                .collect()
                        })
                        .collect();

                    list_sender
                        .send(vec)
                        .expect("Receiver should never be dropped");

                    sender
                        .send(Ok(()))
                        .expect("Receiver should never be dropped");
                }

                TaskManagerCommand::StartProgram { program_name } => {
                    if let Some(program_config) = self.get_program_config(program_name.as_str()) {
                        self.start_program(&program_config).await;
                        sender
                            .send(Ok(()))
                            .expect("Receiver should never be dropped")
                    } else {
                        sender
                            .send(Err(super::ServerCommandError::NoSuchProgram(program_name)))
                            .expect("Receiver should never be dropped")
                    };
                }

                TaskManagerCommand::RestartProgram { program_name } => {
                    if let Some(program_config) = self.get_program_config(program_name.as_str()) {
                        self.stop_program(program_name.as_str()).await;
                        self.start_program(&program_config).await;
                        sender
                            .send(Ok(()))
                            .expect("Receiver should never be dropped")
                    } else {
                        sender
                            .send(Err(super::ServerCommandError::NoSuchProgram(program_name)))
                            .expect("Receiver should never be dropped")
                    };
                }

                TaskManagerCommand::StopProgram { program_name } => {
                    self.stop_program(program_name.as_str()).await;
                    sender
                        .send(Ok(()))
                        .expect("Receiver should never be dropped");
                }

                TaskManagerCommand::SubscribeToProgramEvents {
                    program_name,
                    client,
                } => {
                    if let Some(subscribed_clients) = self.clients.lock().await.get(&program_name) {
                        subscribed_clients.add(client);
                    }
                    sender
                        .send(Ok(()))
                        .expect("Receiver should never be dropped");
                }

                TaskManagerCommand::UnsubscribeToProgramEvents {
                    program_name,
                    client,
                } => {
                    if let Some(subscribed_clients) = self.clients.lock().await.get(&program_name) {
                        subscribed_clients.remove(client);
                    }
                    sender
                        .send(Ok(()))
                        .expect("Receiver should never be dropped");
                }

                TaskManagerCommand::StopAllProcesses => {
                    self.stop_all_processes().await;
                    sender
                        .send(Ok(()))
                        .expect("Receiver should never be dropped");
                }

                TaskManagerCommand::Exit => {
                    self.stop_all_processes().await;
                    sender
                        .send(Ok(()))
                        .expect("Receiver should never be dropped");
                    break;
                }
            }
        }
    }

    async fn stop_all_processes(&mut self) {
        let mut processes = self.processes.lock().await;

        for process in processes
            .iter_mut()
            .flat_map(|(_, process_vec)| process_vec.iter_mut())
        {
            process.stop_and_join_if_running().await;
        }
    }

    async fn stop_program(&mut self, program_name: &str) {
        let mut processes = self.processes.lock().await;

        for process in processes
            .get_mut(program_name)
            .iter_mut()
            .flat_map(|process_vec| process_vec.iter_mut())
        {
            process.stop_and_join_if_running().await;
        }
    }

    fn get_program_config(&self, program_name: &str) -> Option<Arc<ProgramConfig>> {
        for program in self.program_configs.iter() {
            if program.name() == program_name {
                return Some(Arc::clone(program));
            }
        }
        None
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
