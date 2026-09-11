use signal::Signal;
use serde::Serializer;

use libc::unistd::mode_t;

pub fn serialize_signal<S>(signal: &Signal, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&signal.to_string())
}

pub fn serialize_umask<S>(umask: &mode_t, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&format!("{umask:o}"))
}