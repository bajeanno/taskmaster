use super::{ProcessState, ProcessStateChannel};
use crate::process_handler::routine::KillCommandSender;
use derive_getters::Getters;
use tokio::sync::oneshot;
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
        let (s, r): ProcessStateChannel = oneshot::channel();
        if self.kill_command_sender.send(s).await.is_ok() {
            match r.await {
                Ok(response) => match response {
                    ProcessState::Running => {}
                    ProcessState::Stopped => {}
                },
                Err(_) => {
                    todo!("need to log the error and propagate it to any connected CLI"); // this part is to do after the networking module is finished and plugged into the project
                }
            };
        };
        self.join_handle.await.expect("failed to join handle");
    }
}
