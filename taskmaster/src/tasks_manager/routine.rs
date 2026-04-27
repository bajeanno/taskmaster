use super::Process;
use super::TaskManagerCommand;
use super::handle::Handle;
use crate::config::program;
use crate::{CommandReceiver, NominativeStatus};
use crate::{
    config::Program,
    process_handler::{
        self, LogReceiver, LogSender, RoutineSpawnError, Status, StatusReceiver, StatusSender,
    },
};
use std::{collections::HashMap, sync::Arc};
use thiserror::Error;
use tokio::sync::{Mutex, mpsc};

#[derive(Debug, Error)]
enum StartTaskError {
    #[error("")]
    RoutineSpawnError(#[from] RoutineSpawnError),
}

struct Client {}

impl Client {
    fn send(&self) {
        todo!();
    }
}

type ClientMap = Arc<Mutex<HashMap<String, Vec<Client>>>>;

#[allow(dead_code)]
pub struct Routine {
    tasks: Vec<Arc<Program>>,
    clients: ClientMap,
    processes: Arc<Mutex<HashMap<String, Process>>>,
    command_receiver: CommandReceiver,
    log_sender: LogSender,
    status_sender: StatusSender,
}

#[allow(dead_code)] //TODO: remove that
impl Routine {
    pub async fn spawn(tasks: Vec<Arc<Program>>) -> Handle {
        let (log_sender, log_receiver) = mpsc::unbounded_channel();
        let (status_sender, status_receiver) = mpsc::unbounded_channel();
        let (command_sender, command_receiver) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            Self {
                tasks,
                processes: Arc::new(Mutex::new(HashMap::new())),
                clients: Arc::new(Mutex::new(HashMap::new())),
                command_receiver,
                log_sender,
                status_sender,
            }
            .routine(status_receiver, log_receiver)
            .await;
        });

        Handle::new(command_sender)
    }

    async fn routine(mut self, status_receiver: StatusReceiver, log_receiver: LogReceiver) {
        if let Err(_result) = self.start_tasks().await {
            self.stop_all_routines().await
        };
        let handle1 = tokio::spawn(Self::listen_for_logs(log_receiver, self.clients.clone()));
        let handle2 = tokio::spawn(Self::listen_for_status(
            status_receiver,
            Arc::clone(&self.processes),
        ));
        self.event_listener().await;
        handle1.await.expect("listen_for_logs failed");
        handle2.await.expect("listen_for_status failed");
    }

    async fn start_tasks(&mut self) -> Result<(), StartTaskError> {
        for task in self.tasks.clone().iter() {
            self.start_task(task.clone()).await?;
        }
        Ok(())
    }

    async fn start_task(&mut self, task: Arc<Program>) -> Result<(), StartTaskError> {
        let num_procs = *task.num_procs();
        for id in 0..num_procs {
            let task_name = task.name().clone();
            let task_id = task_name.to_owned() + format!("_{}", id).as_str();
            let handle = self
                .start_process(
                    Arc::clone(&task),
                    self.status_sender.clone(),
                    self.log_sender.clone(),
                    task_id.clone(),
                )
                .await?;
            self.processes.lock().await.insert(
                task_id,
                Process {
                    handle,
                    status: Status::Starting,
                },
            );
        }
        Ok(())
    }

    async fn start_process(
        &mut self,
        task: Arc<Program>,
        status_sender: StatusSender,
        log_sender: LogSender,
        task_id: String,
    ) -> Result<process_handler::Handle, StartTaskError> {
        let handle =
            process_handler::Routine::spawn(task, status_sender, log_sender, task_id).await?;
        Ok(handle)
    }

    async fn listen_for_status(
        mut status_receiver: StatusReceiver,
        process_hashmap: Arc<Mutex<HashMap<String, Process>>>,
    ) {
        while let Some(status) = status_receiver.recv().await {
            let mut map = process_hashmap.lock().await;
            if let Some(process) = map.get_mut(&status.process_name) {
                process.status = status.status;
            }
        }
    }

    /// logs are already written to log files, we only need to write them to the client if he asks for it
    async fn listen_for_logs(mut log_receiver: LogReceiver, clients: ClientMap) {
        while let Some(log) = log_receiver.recv().await {
            let index = log
                .process_name
                .rfind('_')
                .or_else(|| Some(log.process_name.len()))
                .expect("split_off on process_name failed");
            //TODO: test that
            if let Some(clients) = clients.lock().await.get(&log.process_name[0..index]) {
                for client in clients {
                    client.send();
                }
            } else {
                todo!()
            }
        }
    }

    async fn event_listener(&mut self) {
        while let Some((command, sender)) = self.command_receiver.recv().await {
            match command {
                TaskManagerCommand::ListTasks(sender) => {
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
                    sender.send(vec).expect("Receiver should never be dropped");
                }

                TaskManagerCommand::Stop { task_name } => {
                    self.stop_task(task_name.as_str()).await;
                }

                TaskManagerCommand::Restart { task_name } => {
                    if let Some(task) = self.get_task(task_name.as_str()) {
                        self.stop_task(task_name.as_str()).await;
                        self.start_task(task).await.unwrap();
                    } else {
                        sender
                            .send(super::ServerCommandError::NoSuchTask(task_name))
                            .expect("Receiver should never be dropped");
                    };
                }

                TaskManagerCommand::Start { task_name } => {
                    if let Some(task) = self.get_task(task_name.as_str()) {
                        self.start_task(task).await.unwrap();
                    } else {
                        sender
                            .send(super::ServerCommandError::NoSuchTask(task_name))
                            .expect("Receiver should never be dropped")
                    };
                }
            }
        }
    }

    /// Can be useful for exit routine
    async fn stop_all_routines(&mut self) {
        for (_, process) in self.processes.lock().await.iter_mut() {
            match process.status {
                Status::Starting | Status::Running => process.stop_process().await,
                _ => {} //routine already stopped (crashed or exited) (do nothing)
            }
        }
    }

    async fn stop_task(&mut self, task_name: &str) {
        for (process_name, process) in self.processes.lock().await.iter_mut() {
            if process_name.starts_with(task_name) {
                process.stop_process().await;
            }
        }
    }

    fn get_task(&self, task_name: &str) -> Option<Arc<Program>> {
        for program in self.tasks.iter() {
            if program.name() == task_name {
                return Some(program.clone());
            }
        }
        None
    }
}
