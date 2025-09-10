use super::super::Message;
use std::fmt::Display;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum CastError {
    SendMessage(mpsc::error::SendError<Message>),
}

impl Display for CastError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SendMessage(error) => {
                write!(f, "Failed to send message to tasks manager: {error}")
            }
        }
    }
}

impl core::error::Error for CastError {}

impl From<mpsc::error::SendError<Message>> for CastError {
    fn from(error: mpsc::error::SendError<Message>) -> Self {
        Self::SendMessage(error)
    }
}
