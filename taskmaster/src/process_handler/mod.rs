mod command;
mod handle;
mod routine;
mod status;
#[cfg(test)]
mod tests;

pub use handle::Handle;

#[allow(unused)]
pub use routine::{
    Log, LogReceiver, LogSender, LogType, Routine, RoutineSpawnError, StatusReceiver,
};
pub use status::{NominativeStatus, Status, StatusSender};
#[allow(unused)]
use std::process::Command;
