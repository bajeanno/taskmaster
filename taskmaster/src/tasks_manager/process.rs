use std::sync::Arc;

use tokio::sync::{Mutex, mpsc::UnboundedSender};

use crate::{
    config::ProgramConfig,
    process_handler::{self, LogSender, NominativeStatus, OutputFile, Status},
};

pub struct Process {
    program_config: Arc<ProgramConfig>,
    handle: Option<process_handler::Handle>,
    pub status: Status,
    process_generation: u32,
    id: usize,
    stderr_file: Arc<Mutex<OutputFile>>,
    stdout_file: Arc<Mutex<OutputFile>>,
}

impl Process {
    pub fn new(
        program: Arc<ProgramConfig>,
        id: usize,
        stderr_file: Arc<Mutex<OutputFile>>,
        stdout_file: Arc<Mutex<OutputFile>>,
    ) -> Self {
        Self {
            program_config: program.clone(),
            handle: None,
            status: Status::default(),
            process_generation: 0,
            id,
            stderr_file,
            stdout_file,
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
        let process_name = format!("{}-{}", self.id, self.program_config.name());
        let handle = process_handler::Routine::spawn(
            self.program_config.clone(),
            status_sender,
            log_sender,
            process_name,
            Arc::clone(&self.stdout_file),
            Arc::clone(&self.stderr_file),
            self.process_generation,
        );

        self.handle = Some(handle);
        self.status = Status::RoutineStarting;
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
