use libc::unistd::mode_t;
use serde::{Deserialize, Deserializer, de};
use signal::Signal;
use std::{str::FromStr, sync::Arc};

use crate::process_handler::OutputFile;

pub fn deserialize_signal<'de, D>(deserializer: D) -> Result<Signal, D::Error>
where
    D: Deserializer<'de>,
{
    let mut signal_str = String::deserialize(deserializer)
        .map_err(|err| serde::de::Error::custom(format!("Failed to parse signal: {err}")))?;
    if !signal_str.starts_with("SIG") {
        signal_str = format!("SIG{signal_str}");
    }
    let signal: Signal = Signal::from_str(signal_str.as_str())
        .map_err(|err| de::Error::custom(format!("Failed to convert signal from string: {err}")))?;
    Ok(signal)
}

pub fn deserialize_umask<'de, D>(deserializer: D) -> Result<mode_t, D::Error>
where
    D: Deserializer<'de>,
{
    let umask_str = String::deserialize(deserializer)
        .map_err(|err| serde::de::Error::custom(format!("Failed to parse umask: {err}")))?;
    let umask = mode_t::from_str_radix(umask_str.as_str(), 8).map_err(|err| {
        serde::de::Error::custom(format!("ParseIntError on umask parsing: {err}"))
    })?;
    if umask > 0o777 { // TODO: Watch out condition in umask rework
        Err(serde::de::Error::custom(
            "umask is greater than 0o777 (max value accepted)",
        ))
    } else {
        Ok(umask)
    }
}

pub fn deserialize_num_procs<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: Deserializer<'de>,
{
    let num_procs = u8::deserialize(deserializer)
        .map_err(|err| serde::de::Error::custom(format!("Failed to parse numprocs: {err}")))?;
    if num_procs == 0 {
        Err(serde::de::Error::custom(
            "Failed to parse numprocs: cannot be 0".to_string(),
        ))
    } else {
        Ok(num_procs)
    }
}

pub fn deserialize_stderr_file<'de, D>(deserializer: D) -> Result<Arc<OutputFile>, D::Error>
where
    D: Deserializer<'de>,
{
    let file_path = String::deserialize(deserializer)
        .map_err(|err| serde::de::Error::custom(format!("Failed to parse stderr file: {err}")))?;
    if file_path.is_empty() {
        return Err(serde::de::Error::custom(
            "Failed to parse stderr file: cannot be empty".to_string(),
        ));
    }

    Ok(Arc::new(
        OutputFile::new_stderr(file_path.as_str()).map_err(|err| {
            serde::de::Error::custom(format!(
                "Failed to open stderr file ({}): {}",
                file_path.as_str(),
                err
            ))
        })?,
    ))
}

pub fn deserialize_stdout_file<'de, D>(deserializer: D) -> Result<Arc<OutputFile>, D::Error>
where
    D: Deserializer<'de>,
{
    let file_path = String::deserialize(deserializer)
        .map_err(|err| serde::de::Error::custom(format!("Failed to parse stdout file: {err}")))?;
    if file_path.is_empty() {
        return Err(serde::de::Error::custom(
            "Failed to parse stdout file: cannot be empty".to_string(),
        ));
    }

    Ok(Arc::new(
        OutputFile::new_stdout(file_path.as_str()).map_err(|err| {
            serde::de::Error::custom(format!(
                "Failed to open stdout file ({}): {}",
                file_path.as_str(),
                err
            ))
        })?,
    ))
}
