use super::Handle;
use super::Message;
use crate::config::Program;
use tokio::sync::mpsc;

pub type Sender = mpsc::Sender<Message>;

pub struct Routine {
    tasks: Vec<Program>,
    receiver: mpsc::Receiver<Message>,
}

impl Routine {
    pub(super) async fn spawn(tasks: Vec<Program>) -> Handle {
        let (sender, receiver) = mpsc::channel(100);

        tokio::spawn(async move {
            Self { tasks, receiver }.event_loop().await;
        });

        Handle::new(sender)
    }

    async fn event_loop(mut self) {
        while let Some(message) = self.receiver.recv().await {
            match message {
                Message::ListTasks(sender) => {
                    sender
                        .send(self.tasks.iter().map(|t| format!("{t}")).collect())
                        .unwrap();
                }
            }
        }
    }
}
