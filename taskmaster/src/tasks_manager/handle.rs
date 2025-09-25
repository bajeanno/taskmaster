use super::Api;
use super::Message;
use super::error::{CallError, CastError, Result};
use super::process;
use tokio::sync::oneshot;

#[derive(Clone)]
pub struct Handle {
    sender: process::Sender,
}

impl Api for Handle {
    async fn list_tasks(&self) -> Result<Vec<String>> {
        self.call(Message::ListTasks).await
    }
}

impl Handle {
    pub(super) fn new(sender: process::Sender) -> Self {
        Self { sender }
    }

    /// Sends a message to the tasks manager process and waits for a response.
    async fn call<Response>(
        &self,
        message_creator: impl FnOnce(oneshot::Sender<Response>) -> Message,
    ) -> Result<Response> {
        let (sender, receiver) = oneshot::channel();

        let message = message_creator(sender);

        self.sender
            .send(message)
            .await
            .map_err(CallError::SendMessage)?;

        Ok(receiver.await.map_err(CallError::ReceiveResponse)?)
    }

    /// Sends a message to the tasks manager process without waiting for a response.
    async fn _cast(&self, message: Message) -> Result<()> {
        Ok(self
            .sender
            .send(message)
            .await
            .map_err(CastError::SendMessage)?)
    }
}
