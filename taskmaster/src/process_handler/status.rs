use std::{fmt::Debug, process::ExitStatus};

#[allow(dead_code)]
pub struct StatusStruct {
    pub process_name: String,
    pub status: Status,
}

#[allow(dead_code)]
pub enum Status {
    Starting,
    Running,
    ErrorDuringStartup { exit_code: u8 },
    FailedToSpawn(tokio::io::Error),
    Exited(ExitStatus),
}

impl Debug for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Starting => write!(f, "Status::Starting"),
            Status::Running => write!(f, "Status::Running"),
            Status::Exited(_) => write!(f, "Status::Exited"),
            Status::FailedToSpawn(_) => write!(f, "Status::FailedToSpawn"),
            Status::ErrorDuringStartup { exit_code } => {
                write!(f, "Status::ErrorDuringStartup{{ exit_code = {exit_code} }}")
            }
        }
    }
}
