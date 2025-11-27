use crate::{
    client_handler::ClientId,
    process_handler::routine::{LogReceiver, StatusReceiver},
};
use derive_getters::Getters;
use tokio::task::JoinHandle as TokioJoinHandle;

type JoinHandle = TokioJoinHandle<()>;

#[derive(Getters)]
#[allow(dead_code)] //TODO: Remove that
pub struct Handle {
    pub join_handle: JoinHandle,
    pub status_receiver: StatusReceiver,
    pub log_receiver: LogReceiver,
}

#[allow(dead_code)] //TODO: Remove that
impl Handle {
    pub fn new(
        join_handle: tokio::task::JoinHandle<()>,
        status_receiver: StatusReceiver,
        log_receiver: LogReceiver,
    ) -> Self {
        Self {
            join_handle,
            status_receiver,
            log_receiver,
        }
    }

    pub async fn attach(&self, _client: ClientId) {
        todo!()
    }

    pub async fn detach(&self) {
        todo!()
    }
}
