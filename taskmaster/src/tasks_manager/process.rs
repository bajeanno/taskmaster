use std::{
    fmt::Debug,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use tokio::sync::mpsc::UnboundedSender;

use crate::{
    config::ProgramConfig,
    process_handler::{self, LogSender, NominativeStatus, Status},
};

pub struct Process {
    program_config: Arc<ProgramConfig>,
    handle: Option<process_handler::Handle>,
    pub nominative_status: NominativeStatus,
    instance_id: u64,
    process_name: String,
}

impl Debug for Process {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Process")
            .field("nominative_status", &self.nominative_status)
            .field("instance_id", &self.instance_id)
            .finish()
    }
}

static NEXT_PROCESS_INSTANCE_ID: AtomicU64 = AtomicU64::new(1);

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
            instance_id: 0,
            process_name,
        }
    }

    pub fn auto_start(
        mut self,
        status_sender: &UnboundedSender<NominativeStatus>,
        log_sender: &LogSender,
    ) -> Self {
        if *self.program_config.auto_start() {
            self.start(status_sender, log_sender);
        }
        self
    }

    pub fn auto_start_on_reload(
        &mut self,
        status_sender: &UnboundedSender<NominativeStatus>,
        log_sender: &LogSender,
    ) {
        if *self.program_config.auto_start_on_reload() {
            self.start(status_sender, log_sender);
        }
    }

    pub fn start(
        &mut self,
        status_sender: &UnboundedSender<NominativeStatus>,
        log_sender: &LogSender,
    ) {
        if self.handle.is_some() {
            return;
        }
        self.instance_id = NEXT_PROCESS_INSTANCE_ID.fetch_add(1, Ordering::Relaxed);
        let handle = process_handler::Routine::spawn(
            self.program_config.clone(),
            status_sender.clone(),
            log_sender.clone(),
            self.process_name.clone(),
            self.instance_id,
        );

        self.handle = Some(handle);
        self.nominative_status.status = Status::RoutineStarting;
    }

    pub async fn update_program_config(&mut self, new_config: Arc<ProgramConfig>) {
        self.program_config = new_config;

        if let Some(handle) = &self.handle {
            handle
                .send_reloaded_config(Arc::clone(&self.program_config))
                .await;
        };
    }

    pub fn instance_id(&self) -> u64 {
        self.instance_id
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

    #[cfg(test)]
    pub fn is_running(&self) -> bool {
        self.handle.is_some()
    }

    #[cfg(test)]
    pub fn program_config(&self) -> Arc<ProgramConfig> {
        Arc::clone(&self.program_config)
    }
}
