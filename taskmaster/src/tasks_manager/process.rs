use crate::process_handler::{self, ProcessStateChannel, Status};
use tokio::sync::oneshot;
pub struct Process {
    pub(super) handle: process_handler::Handle,
    pub(super) status: Status,
}

impl Process {
    /// Stops a routine by sending a kill command.
    pub(super) async fn stop_process(&mut self) {
        let (s, r): ProcessStateChannel = oneshot::channel();
        let _ = self.handle.kill_command_sender.send(s).await; // thows an error on dropped receiver, the error is silenced
        match r.await {
            Ok(response) => match response {
                process_handler::ProcessState::Running => {}
                process_handler::ProcessState::Stopped => {}
            },
            Err(_) => {
                todo!("need to log the error and propagate it to any connected CLI"); // this part is to do after the networking module is finished and plugged into the project
            }
        }
    }
}
