use std::str::FromStr;
use serde::{Deserialize, Deserializer, de};
use signal::Signal;
use libc::unistd::mode_t;

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
    if umask > 0o777 {
        Err(serde::de::Error::custom(
            "umask is greater than 0o777 (max value accepted)",
        ))
    } else {
        Ok(umask)
    }
}
