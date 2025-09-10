use tokio::sync::oneshot;

#[derive(Debug)]
pub enum Message {
    ListTasks(oneshot::Sender<Vec<String>>),
}
