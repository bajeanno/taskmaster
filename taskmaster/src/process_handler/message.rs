use tokio::sync::mpsc::Sender;

use crate::process_handler::Status;

#[allow(dead_code)]
pub enum Message {
    Stop,
    Restart,
    Start,
    Status(Sender<Status>),
}
