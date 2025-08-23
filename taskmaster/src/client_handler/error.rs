use std::fmt::Display;

use commands::ServerCommand;

use crate::{client_handler::ClientId, tasks_manager};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    #[allow(dead_code)]
    ReadFrame {
        client_id: ClientId,
        error: connection::Error,
    },

    #[allow(dead_code)]
    WriteFrame {
        client_id: ClientId,
        error: connection::Error,
    },

    #[allow(dead_code)]
    HandleCommand {
        client_id: ClientId,
        command: ServerCommand,
        error: tasks_manager::Error,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl core::error::Error for Error {}
