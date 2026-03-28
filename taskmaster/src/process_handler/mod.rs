mod command;
mod handle;
mod routine;
mod status;
#[cfg(test)]
mod tests;

pub use handle::Handle;
#[allow(unused)]
pub use routine::{
    KillCommandSender, Log, LogReceiver, LogSender, LogType, ProcessState, ProcessStateChannel,
    Routine, RoutineSpawnError, StatusReceiver, StatusSender,
};
pub use status::Status;
#[allow(unused)]
use std::process::Command;
