use crate::{
    config::Program,
    process_handler::{
        self, KillCommandSender, LogReceiver, LogSender, Status, StatusReceiver, StatusSender,
    },
};

use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
mod handle;
mod state_machine;
use crate::CommandReceiver;
pub use handle::Handle;
use tokio::sync::mpsc;

#[allow(dead_code)]
pub struct Routine {
    tasks: Vec<Arc<Program>>,
    hashmap_status: Arc<Mutex<HashMap<String, Status>>>,
    stop_senders: HashMap<String, Vec<KillCommandSender>>,
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
                stop_senders: HashMap::new(),
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
        for task in self.tasks.iter() {
            let num_procs = task.num_procs().clone();

            for i in 0..num_procs {
                let handle = process_handler::Routine::spawn(
                    Arc::clone(task),
                    status_sender.clone(),
                    log_sender.clone(),
                    task.name().to_owned() + format!("_{}", i).as_str(),
                )
                .await
                .unwrap();
                self.stop_senders
                    .entry(task.name().clone())
                    .and_modify(|vec| {
                        vec.push(handle.kill_command_sender.clone());
                    })
                    .or_insert(vec![handle.kill_command_sender]);
            }
        }

        let handle1 = tokio::spawn(Self::listen_for_logs(log_receiver));
        let handle2 = tokio::spawn(Self::listen_for_status(
            status_receiver,
            Arc::clone(&self.hashmap_status),
        ));

        self.event_listener().await;

        handle1.await.expect("listen_for_logs failed");
        handle2.await.expect("listen_for_status failed");
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
                    self.stop_senders.get(&process_name);
                }
                commands::ServerCommand::Restart { process_name } => {
                    todo!("Restart {}", process_name)
                }
                commands::ServerCommand::Start { process_name } => todo!("Start {}", process_name),
            }
        }
    }
}
