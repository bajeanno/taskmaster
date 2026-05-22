use crate::process_handler::routine::KillCommandSender;
use derive_getters::Getters;
use tokio::task::JoinHandle as TokioJoinHandle;

type JoinHandle = TokioJoinHandle<()>;

#[derive(Getters)]
#[allow(dead_code)] //TODO: Remove that
pub struct Handle {
    pub join_handle: JoinHandle,
    pub kill_command_sender: KillCommandSender,
}

#[allow(dead_code)] //TODO: Remove that
impl Handle {
    pub(super) fn new(
        join_handle: tokio::task::JoinHandle<()>,
        kill_command_sender: KillCommandSender,
    ) -> Self {
        Self {
            join_handle,
            kill_command_sender,
        }
    }

    pub async fn stop(self) {
        let result = self.kill_command_sender.send(()).await;
        let join_handle_result = self.join_handle.await;

        result.expect("receiver should never be dropped");
        join_handle_result.expect("failed to join handle");
    }
}
