use std::sync::Arc;

use crate::{
    config::ProgramConfig,
    process_handler::{KillCommandSender, ReloadEventSender},
};
use tokio::task::JoinHandle as TokioJoinHandle;

type JoinHandle = TokioJoinHandle<()>;

#[allow(dead_code)] //TODO: Remove that
#[cfg_attr(test, derive(Debug))]
pub struct Handle {
    join_handle: JoinHandle,
    kill_command_sender: KillCommandSender,
    reload_event_sender: ReloadEventSender,
}

#[allow(dead_code)] //TODO: Remove that
impl Handle {
    pub(super) fn new(
        join_handle: tokio::task::JoinHandle<()>,
        kill_command_sender: KillCommandSender,
        reload_event_sender: ReloadEventSender,
    ) -> Self {
        Self {
            join_handle,
            kill_command_sender,
            reload_event_sender,
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

    pub async fn send_reloaded_config(&self, reloaded_config: Arc<ProgramConfig>) {
        self.reload_event_sender
            .send(reloaded_config)
            .await
            .expect("failed to send reloaded config");
    }
}
