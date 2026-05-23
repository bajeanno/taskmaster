use crate::process_handler::{self, Status};

pub struct Process {
    handle: Option<process_handler::Handle>,
    pub(super) status: Status,
}

impl Process {
    pub(super) fn new(handle: Option<process_handler::Handle>, status: Status) -> Self {
        Self { handle, status }
    }

    pub(super) fn is_async_task_running(&self) -> bool {
        self.handle.is_some()
    }

    /// Stops a routine by sending a kill command.
    pub(super) async fn stop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.join().await;
        }
    }
}
