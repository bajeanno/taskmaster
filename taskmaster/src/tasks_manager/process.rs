use std::sync::Arc;

use tokio::sync::mpsc::UnboundedSender;

use crate::{
    config::ProgramConfig,
    process_handler::{self, LogSender, NominativeStatus, Status},
};

pub struct Process {
    program_config: Arc<ProgramConfig>,
    handle: Option<process_handler::Handle>,
    pub nominative_status: NominativeStatus,
    process_generation: u32,
    process_name: String,
}

impl Process {
    pub fn new(program: Arc<ProgramConfig>, id: usize) -> Self {
        let process_name = format!("{}-{}", program.name(), id);
        Self {
            program_config: Arc::clone(&program),
            handle: None,
            nominative_status: NominativeStatus {
                process_name: process_name.clone(),
                status: Status::default(),
            },
            process_generation: 0,
            process_name,
        }
    }

    pub fn auto_start(
        mut self,
        status_sender: UnboundedSender<NominativeStatus>,
        log_sender: LogSender,
    ) -> Self {
        if *self.program_config.auto_start() {
            self.start(status_sender, log_sender);
        }
        self
    }

    pub fn start(
        &mut self,
        status_sender: UnboundedSender<NominativeStatus>,
        log_sender: LogSender,
    ) {
        if self.handle.is_some() {
            return;
        }
        let handle = process_handler::Routine::spawn(
            self.program_config.clone(),
            status_sender,
            log_sender,
            self.process_name.clone(),
            self.process_generation,
        );

        self.handle = Some(handle);
        self.nominative_status.status = Status::RoutineStarting;
    }

    pub fn process_generation(&self) -> u32 {
        self.process_generation
    }

    /// Stops a routine by sending a kill command.
    pub async fn stop_and_join_if_running(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.stop_and_join().await
        }
        self.process_generation = self.process_generation.wrapping_add(1);
    }

    pub async fn join_if_running(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.join().await;
        }
    }
}
