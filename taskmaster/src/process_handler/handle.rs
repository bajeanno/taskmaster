use crate::process_handler::routine::{KillCommandSender, LogReceiver, StatusReceiver};
use derive_getters::Getters;
use tokio::task::JoinHandle as TokioJoinHandle;

type JoinHandle = TokioJoinHandle<()>;

#[derive(Getters)]
#[allow(dead_code)] //TODO: Remove that
pub struct Handle {
    pub join_handle: JoinHandle,
    pub status_receiver: StatusReceiver,
    pub log_receiver: LogReceiver,
    pub kill_command_sender: KillCommandSender,
}

#[allow(dead_code)] //TODO: Remove that
impl Handle {
    pub(super) fn new(
        join_handle: tokio::task::JoinHandle<()>,
        status_receiver: StatusReceiver,
        log_receiver: LogReceiver,
        kill_command_sender: KillCommandSender,
    ) -> Self {
        Self {
            join_handle,
            status_receiver,
            log_receiver,
            kill_command_sender,
        }
    }
}
