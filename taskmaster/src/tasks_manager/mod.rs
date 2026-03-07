mod api;
pub use api::Api;
#[cfg(test)]
pub use api::MockApi;

use crate::config::Program;

pub struct Routine {
    tasks: Vec<Program>,
}

impl Routine {
    pub(super) async fn spawn(tasks: Vec<Program>) -> Handle {
        let (sender, receiver) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            Self { tasks, receiver }.event_loop().await;
        });

        Handle::new(sender)
    }
}
