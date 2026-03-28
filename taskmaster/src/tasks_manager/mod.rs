use crate::{
    config::Program,
    process_handler::{
        self, LogReceiver, LogSender, ProcessStateChannel, RoutineSpawnError, Status,
        StatusReceiver, StatusSender,
    },
};
use std::{collections::HashMap, sync::Arc};
use thiserror::Error;
use tokio::sync::{Mutex, oneshot};
mod handle;
mod state_machine;
use crate::CommandReceiver;
pub use handle::Handle;
use tokio::sync::mpsc;

#[derive(Debug, Error)]
enum StartTaskError {
    #[error("")]
    RoutineSpawnError(#[from] RoutineSpawnError),
}

#[allow(dead_code)]
pub struct Routine {
    tasks: Vec<Arc<Program>>,
    hashmap_status: Arc<Mutex<HashMap<String, Status>>>,
    handles: HashMap<String, Vec<process_handler::Handle>>,
    command_receiver: CommandReceiver,
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
                hashmap_status: Arc::new(Mutex::new(HashMap::new())),
                handles: HashMap::new(),
                command_receiver,
            }
            .routine(log_sender, log_receiver, status_sender, status_receiver)
            .await;
        });

        Handle::new(command_sender)
    }

    async fn routine(
        mut self,
        log_sender: LogSender,
        log_receiver: LogReceiver,
        status_sender: StatusSender,
        status_receiver: StatusReceiver,
    ) {
        if let Err(_result) = self.start_tasks(status_sender, log_sender).await {
            self.stop_routines().await
        };
        let handle1 = tokio::spawn(Self::listen_for_logs(log_receiver));
        let handle2 = tokio::spawn(Self::listen_for_status(
            status_receiver,
            Arc::clone(&self.hashmap_status),
        ));

        self.event_listener().await;

        handle1.await.expect("listen_for_logs failed");
        handle2.await.expect("listen_for_status failed");
    }

    async fn start_tasks(
        &mut self,
        status_sender: StatusSender,
        log_sender: LogSender,
    ) -> Result<(), StartTaskError> {
        for task in self.tasks.clone().iter() {
            let num_procs = *task.num_procs();

            for id in 0..num_procs {
                self.start_task(
                    Arc::clone(task),
                    status_sender.clone(),
                    log_sender.clone(),
                    id,
                )
                .await?;
            }
        }
        Ok(())
    }

    async fn start_task(
        &mut self,
        task: Arc<Program>,
        status_sender: StatusSender,
        log_sender: LogSender,
        id: u32,
    ) -> Result<(), StartTaskError> {
        let task_name = task.name().clone();
        let task_id = task_name.to_owned() + format!("_{}", id).as_str();
        let handle =
            process_handler::Routine::spawn(task, status_sender, log_sender, task_id).await?;
        self.handles.entry(task_name.clone()).or_default();
        self.handles.get_mut(&task_name).unwrap().push(handle);
        Ok(())
    }

    async fn listen_for_status(
        mut status_receiver: StatusReceiver,
        hashmap_status: Arc<Mutex<HashMap<String, Status>>>,
    ) {
        while let Some(status_struct) = status_receiver.recv().await {
            hashmap_status
                .lock()
                .await
                .insert(status_struct.process_name, status_struct.status);
        }
    }

    /// logs are already written to log files, we only need to write them to the client if he asks for it
    async fn listen_for_logs(mut log_receiver: LogReceiver) {
        while let Some(_log) = log_receiver.recv().await {
            todo!() //TODO: do something with logs
        }
    }

    async fn event_listener(&mut self) {
        while let Some(command) = self.command_receiver.recv().await {
            match command {
                commands::ServerCommand::ListTasks => todo!("ListTasks (status command)"),
                commands::ServerCommand::Stop { process_name } => {
                    self.handles.get(&process_name);
                    todo!()
                }
                commands::ServerCommand::Restart { process_name } => {
                    todo!("Restart {}", process_name)
                }
                commands::ServerCommand::Start { process_name } => todo!("Start {}", process_name),
            }
        }
    }

    async fn stop_routines(&mut self) {
        for (_, handles) in self.handles.iter_mut() {
            for handle in handles.iter_mut() {
                Self::stop_routine(handle).await;
            }
        }
    }

    async fn stop_routine(handle: &mut process_handler::Handle) {
        let (s, r): ProcessStateChannel = oneshot::channel();
        handle.kill_command_sender.send(s).await;
        r.await; //TODO: handle response
    }
}
