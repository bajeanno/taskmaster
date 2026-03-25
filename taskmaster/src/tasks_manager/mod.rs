use crate::{
    config::Program,
    process_handler::{LogReceiver, LogSender, Status, StatusReceiver, StatusSender},
};
use std::{collections::HashMap, sync::Arc};
mod handle;
mod state_machine;
pub use handle::Handle;
use tokio::sync::mpsc;

#[allow(dead_code)]
pub struct Routine {
    tasks: Vec<Arc<Program>>,
    statuses: HashMap<String, Vec<Status>>,
    log_sender: LogSender,
    log_receiver: LogReceiver,
    status_sender: StatusSender,
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
                log_sender,
                log_receiver,
                status_sender,
                status_receiver,
            }
            .routine()
            .await;
        });

        Handle::new()
    }

    async fn routine(&self) {
        // for each process, clone sender and Arc<Program>
        // spawn routines
        // listen for status and logs
        //
        // may need an id per process to sign each log / status
    }
}
