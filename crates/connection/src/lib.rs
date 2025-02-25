mod connection;
pub use connection::Connection;

mod error;
pub(crate) use error::FrameDecodeError;
pub use error::{Error, Result};
