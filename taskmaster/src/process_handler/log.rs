use std::sync::Arc;

use crate::{output_file::OutputFile, process::ProcessId, process_handler::LogSender};

#[derive(Debug, Clone, Copy)]
pub enum LogType {
    Stdout,
    Stderr,
}

#[derive(Clone, Debug)]
pub struct Log {
    pub message: String,
    pub process_id: ProcessId,
    pub log_type: LogType,
}

impl Log {
    pub fn new(log_type: LogType, buffer: &[u8], id: &ProcessId) -> Self {
        match log_type {
            LogType::Stdout => Log {
                message: format!(
                    "{}-{}: {}",
                    id.task_name,
                    id.id,
                    String::from_utf8_lossy(buffer)
                ),
                process_id: id.clone(),
                log_type,
            },
            LogType::Stderr => Log {
                message: format!(
                    "{}-{}: {}",
                    id.task_name,
                    id.id,
                    String::from_utf8_lossy(buffer)
                ),
                process_id: id.clone(),
                log_type,
            },
        }
    }
    /// Sends a log message over the channel and writes it to the appropriate output file.
    /// This function performs two operations:
    /// - Write the log message to the corresponding output file (stdout or stderr)
    /// - Send the log message through the log channel to any receivers
    ///
    /// # Arguments
    ///
    /// * `log` - A `Log` struct containing the log type, the task's name and the log itself
    /// * `log_sender` - A `mpsc::Sender<Log>` to send log to the manager coroutine
    /// * `output` - A `OutputFile` enum that contains the file to write in
    ///
    /// # Panics
    ///
    /// Will panic if the `OutputFile` and the `LogType` enums are not accorded.
    /// That should never happen because those structs are both constructed side by side.
    ///
    pub async fn dispatch_log(self, log_sender: &mut LogSender, output: Arc<OutputFile>) {
        //TODO: move this to task_manager
        output.write(&self).await;
        let process_name = self.process_id.task_name.clone();
        log_sender
            .send(self)
            .inspect_err(|_| {
                eprintln!(
                    "Taskmaster error: {}: Log receiver was dropped",
                    process_name
                )
            })
            .unwrap()
    }
}
