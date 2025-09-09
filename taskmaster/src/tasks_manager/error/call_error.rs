use super::super::Message;
use std::fmt::Display;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
pub enum CallError {
    SendMessage(mpsc::error::SendError<Message>),
    ReceiveResponse(oneshot::error::RecvError),
}

impl Display for CallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SendMessage(error) => {
                write!(f, "Failed to send message to tasks manager: {error}")
            }
            Self::ReceiveResponse(error) => {
                write!(f, "Failed to receive response from tasks manager: {error}")
            }
        }
    }
}

impl core::error::Error for CallError {}

impl From<mpsc::error::SendError<Message>> for CallError {
    fn from(error: mpsc::error::SendError<Message>) -> Self {
        Self::SendMessage(error)
    }
}

impl From<oneshot::error::RecvError> for CallError {
    fn from(error: oneshot::error::RecvError) -> Self {
        Self::ReceiveResponse(error)
    }
}
