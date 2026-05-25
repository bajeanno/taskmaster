use crate::process_handler::{self, Status};

#[derive(Default)]
pub struct Process {
    handle: Option<process_handler::Handle>,
    pub status: Status,
    process_generation: u32,
}

impl Process {
    pub fn new(
        handle: Option<process_handler::Handle>,
        status: Status,
        process_generation: u32,
    ) -> Self {
        Self {
            handle,
            status,
            process_generation,
        }
    }

    pub fn process_generation(&self) -> u32 {
        self.process_generation
    }

    pub fn is_async_task_running(&self) -> bool {
        self.handle.is_some()
    }

    /// Stops a routine by sending a kill command.
    pub async fn stop_and_join_if_running(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.stop_and_join().await
        }
    }

    pub async fn join_if_running(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.join().await;
        }
    }
}
