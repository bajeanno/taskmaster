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
    processes: Arc<Mutex<HashMap<String, Process>>>,
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
        let num_procs = *program_config.num_procs();

        for id in 0..num_procs {
            let program_name = program_config.name();
            let process_id = format!("{program_name}-{id}");
            self.start_process(process_id, Arc::clone(program_config))
                .await;
        }
    }

    async fn start_process(&mut self, process_id: String, program_config: Arc<ProgramConfig>) {
        match self.processes.lock().await.entry(process_id.clone()) {
            hash_map::Entry::Occupied(mut entry) => {
                if !entry.get().is_async_task_running() {
                    let process_generation = entry.get().process_generation().wrapping_add(1);

                    *entry.get_mut() = self
                        .start_process_handler_routine(
                            program_config,
                            process_id,
                            process_generation,
                        )
                        .await
                }
            }
            hash_map::Entry::Vacant(entry) => {
                entry.insert(
                    self.start_process_handler_routine(program_config, process_id, 0)
                        .await,
                );
            }
        };
    }

    async fn start_process_handler_routine(
        &self,
        program_config: Arc<ProgramConfig>,
        process_id: String,
        process_generation: u32,
    ) -> Process {
        match process_handler::Routine::spawn(
            program_config,
            self.status_sender.clone(),
            self.log_sender.clone(),
            process_id,
            process_generation,
        )
        .await
        {
            Ok(handle) => Process::new(Some(handle), Status::Starting, process_generation),
            Err(err) => Process::new(None, Status::FailedToSpawnRoutine(err), process_generation),
        }
    }

    async fn listen_for_status(
        mut status_receiver: StatusReceiver,
        process_hashmap: Arc<Mutex<HashMap<String, Process>>>,
    ) {
        while let Some(nominative_status) = status_receiver.recv().await {
            let mut map = process_hashmap.lock().await;
            if let Some(process) = map.get_mut(&nominative_status.process_name) {
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
                    let vec: Vec<NominativeStatus> = self
                        .processes
                        .lock()
                        .await
                        .iter()
                        .map(|(name, process)| NominativeStatus {
                            process_name: name.clone(),
                            status: process.status.clone(),
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
        for (_, process) in self.processes.lock().await.iter_mut() {
            process.stop_and_join_if_running().await;
        }
    }

    async fn stop_program(&mut self, program_name: &str) {
        for (process_name, process) in self.processes.lock().await.iter_mut() {
            if process_name.starts_with(program_name) {
                //TODO: change start_with call, invalid
                process.stop_and_join_if_running().await;
            }
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
