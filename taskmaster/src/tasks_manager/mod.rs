mod handle;
pub use handle::Handle;

mod message;
use message::Message;

mod process;
use process::Process;

mod error;
pub use error::Error;
use error::Result;

mod api;
pub use api::Api;
#[cfg(test)]
pub use api::MockApi;

use crate::parser::program::Program;

pub async fn spawn(tasks: Vec<Program>) -> Handle {
    Process::spawn(tasks).await
}
