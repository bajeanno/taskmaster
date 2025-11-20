use crate::{client_handler::ClientId, process_handler::routine::Receiver};
use derive_getters::Getters;
use tokio::task::JoinHandle as TokioJoinHandle;

type JoinHandle = TokioJoinHandle<()>;

#[derive(Getters)]
#[allow(dead_code)] //TODO: Remove that
pub struct Handle {
    pub join_handle: JoinHandle,
    receiver: Receiver,
}

#[allow(dead_code)] //TODO: Remove that
impl Handle {
    pub fn new(join_handle: tokio::task::JoinHandle<()>, receiver: Receiver) -> Self {
        Self {
            join_handle,
            receiver,
        }
    }

    pub async fn attach(&self, _client: ClientId) {
        todo!()
    }

    pub async fn detach(&self) {
        todo!()
    }
}
