use serde::Serializer;
use signal::Signal;

pub fn serialize_signal<S>(signal: &Signal, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(signal.as_ref())
}
