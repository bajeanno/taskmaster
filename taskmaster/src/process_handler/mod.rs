mod command;
mod handle;
mod routine;
mod status;
#[cfg(test)]
mod tests;

pub use handle::Handle;
#[allow(unused)]
pub use routine::{
    Log, LogReceiver, LogSender, LogType, Routine, RoutineSpawnError, StatusReceiver, StatusSender,
};
pub use status::{NominativeStatus, Status};
#[allow(unused)]
use std::process::Command;
