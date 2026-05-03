use crate::process_handler::{self, Status};
pub struct Process {
    pub(super) handle: Option<process_handler::Handle>,
    pub(super) status: Status,
}

impl Process {
    /// Stops a routine by sending a kill command.
    pub(super) async fn stop_process(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.stop().await;
        }
    }
}
