mod handle;
pub use handle::Handle;

mod message;
use message::Message;

mod routine;
use routine::Routine;

mod error;
pub use error::Error;
use error::Result;

mod api;
pub use api::Api;
#[cfg(test)]
pub use api::MockApi;

use crate::config::Program;

pub async fn spawn(tasks: Vec<Program>) -> Handle {
    Routine::spawn(tasks).await
}
