#[allow(clippy::module_inception)]
mod client_handler;
pub use client_handler::ClientHandler;

mod error;
pub use error::{Error, Result};

mod commands;

#[cfg(test)]
mod test_utils;
