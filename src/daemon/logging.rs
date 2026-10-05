use crate::daemon::Error;
use std::{
    fs::{File, OpenOptions},
    sync::{LazyLock, Mutex},
};

const LOG_FILE_PATH: &str = "/var/log/taskmaster.log";
static LOG_FILE: LazyLock<Mutex<Option<File>>> = LazyLock::new(|| Mutex::new(None));

pub fn inspect_log_file_error() -> Result<(), Error> {
    *LOG_FILE.lock().unwrap() = Some(
        OpenOptions::new()
            .create(true)
            .append(true)
            .write(true)
            .truncate(false)
            .open(LOG_FILE_PATH)
            .map_err(Error::LogFile)?,
    );
    Ok(())
}

pub fn log(_message: &str) -> std::io::Result<()> {
    // TODO: create time to str and write message to file
    Ok(())
}
