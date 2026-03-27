use crate::{
    config::Program,
    process_handler::{
        self, KillCommandSender, LogReceiver, LogSender, Status, StatusReceiver, StatusSender,
    },
};
use std::{collections::HashMap, sync::Arc};
mod handle;
mod state_machine;
pub use handle::Handle;
use tokio::sync::mpsc;

#[allow(dead_code)]
pub struct Routine {
    tasks: Vec<Arc<Program>>,
    statuses: HashMap<String, Status>,
    stop_senders: HashMap<String, Vec<KillCommandSender>>,
    log_receiver: LogReceiver,
    status_receiver: StatusReceiver,
}

#[allow(dead_code)] //TODO: remove that
impl Routine {
    pub async fn spawn(tasks: Vec<Arc<Program>>) -> Handle {
        let (log_sender, log_receiver) = mpsc::unbounded_channel();
        let (status_sender, status_receiver) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            Self {
                tasks,
                statuses: HashMap::new(),
                stop_senders: HashMap::new(),
                log_receiver,
                status_receiver,
            }
            .routine(log_sender, status_sender)
            .await;
        });

        Handle::new()
    }

    async fn routine(mut self, log_sender: LogSender, status_sender: StatusSender) {
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
                .unwrap(); //TODO: remove that and do proper error handling
                self.stop_senders
                    .entry(task.name().clone())
                    .and_modify(|vec| {
                        vec.push(handle.kill_command_sender.clone());
                    })
                    .or_insert(vec![handle.kill_command_sender]);
            }
        }
        // listen for status and logs
        //
        // may need an id per process to sign each log / status
    }

    /// logs are already written to log files, we only need to write them to the client if he asks for it
    async fn listen_for_logs(&self) {
        todo!()
    }

    async fn listen_for_status(&mut self) {
        while let Some(status_struct) = self.status_receiver.recv().await {
            self.statuses
                .insert(status_struct.process_name, status_struct.status);
        }
    }
}
