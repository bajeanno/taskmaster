mod command;
mod handle;
mod log;
mod routine;
mod status;
#[cfg(test)]
mod tests;

use std::sync::Arc;

pub use handle::Handle;

#[allow(unused)]
pub use routine::{Routine, RoutineSpawnError};
pub use status::{NominativeStatus, Status, StatusSender};
#[allow(unused)]
use tokio::process::Command;
use tokio::process::{ChildStderr, ChildStdout};
use tokio::{io::BufReader, process::Child, sync::mpsc};

use crate::config::ProgramConfig;
pub use log::{Log, LogType};

pub type LogReceiver = mpsc::UnboundedReceiver<Log>;
pub type LogSender = mpsc::UnboundedSender<Log>;

pub type StatusReceiver = mpsc::UnboundedReceiver<NominativeStatus>;

pub type KillCommandReceiver = mpsc::Receiver<()>;
pub type KillCommandSender = mpsc::Sender<()>;

pub type ReloadEventReceiver = mpsc::Receiver<Arc<ProgramConfig>>;
pub type ReloadEventSender = mpsc::Sender<Arc<ProgramConfig>>;

pub struct Outputs {
    stdout: BufReader<ChildStdout>,
    stderr: BufReader<ChildStderr>,
}

impl Outputs {
    pub fn new(child: &mut Child) -> Self {
        Self {
            stdout: BufReader::new(
                child
                    .stdout
                    .take()
                    .expect("Child process stdout not captured"),
            ),
            stderr: BufReader::new(
                child
                    .stderr
                    .take()
                    .expect("Child process stderr not captured"),
            ),
        }
    }
}
