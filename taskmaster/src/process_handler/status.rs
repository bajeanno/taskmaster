use std::process::ExitStatus;

#[allow(dead_code)]
#[derive(Clone)]
pub enum Status {
    NotSpawned,
    Starting,
    Running,
    FailedToInit {
        error_message: String,
        exit_code: u8,
    },
    FailedToSpawn {
        error_message: String,
    },
    Exited(ExitStatus),
}
