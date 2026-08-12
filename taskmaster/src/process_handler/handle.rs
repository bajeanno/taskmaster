use crate::process_handler::KillCommandSender;
use tokio::task::JoinHandle as TokioJoinHandle;

type JoinHandle = TokioJoinHandle<()>;

#[allow(dead_code)] //TODO: Remove that
#[cfg_attr(test, derive(Debug))]
pub struct Handle {
    join_handle: JoinHandle,
    kill_command_sender: KillCommandSender,
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

    pub async fn stop_and_join(self) {
        // Ignoring result as the subprocess can end between the moment
        // we verified it's alive and the moment we sent the kill command to the sub-routine
        let _ = self.kill_command_sender.send(()).await;

        self.join().await
    }

    pub async fn join(self) {
        self.join_handle.await.expect("failed to join handle");
    }
}
