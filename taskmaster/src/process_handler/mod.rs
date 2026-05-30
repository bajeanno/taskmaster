mod command;
mod handle;
mod routine;
mod status;
#[cfg(test)]
mod tests;

pub use handle::Handle;

use tokio::{fs::File, io::BufReader, process::Child, sync::mpsc};

#[allow(unused)]
pub use routine::{Routine, RoutineSpawnError};
pub use status::{NominativeStatus, Status, StatusSender};
#[allow(unused)]
use tokio::process::Command;
use tokio::process::{ChildStderr, ChildStdout};

#[derive(Clone, Debug)]
pub enum LogType {
    Stdout,
    Stderr,
}

#[derive(Clone, Debug)]
pub struct Log {
    pub message: String,
    pub process_name: String,
    pub log_type: LogType,
}

impl Log {
    fn new(output_file: &OutputFile, buffer: &[u8], name: &str) -> Self {
        match output_file {
            OutputFile::Stdout(_) => Log {
                message: format!("{}: {}", name, String::from_utf8_lossy(buffer)),
                process_name: name.to_string(),
                log_type: LogType::Stdout,
            },
            OutputFile::Stderr(_) => Log {
                message: format!("{}: {}", name, String::from_utf8_lossy(buffer)),
                process_name: name.to_string(),
                log_type: LogType::Stderr,
            },
        }
    }
}

pub type LogReceiver = mpsc::UnboundedReceiver<Log>;
pub type LogSender = mpsc::UnboundedSender<Log>;

pub type StatusReceiver = mpsc::UnboundedReceiver<NominativeStatus>;

pub type KillCommandReceiver = mpsc::Receiver<()>;
pub type KillCommandSender = mpsc::Sender<()>;

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
enum OutputFile {
    Stdout(File),
    Stderr(File),
}
