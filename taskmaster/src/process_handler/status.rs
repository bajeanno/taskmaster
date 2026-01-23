use std::process::ExitStatus;

#[allow(dead_code)]
pub enum Status {
    NotSpawned,
    Starting,
    Running,
    ErrorDuringStartup { exit_code: u8 },
    FailedToSpawn(tokio::io::Error),
    Exited(ExitStatus),
}
