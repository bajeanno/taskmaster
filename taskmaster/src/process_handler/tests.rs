use std::time::Duration;
#[allow(unused)]
use crate::Program;
#[allow(unused)]
use crate::process_handler::Routine;
#[allow(unused)]
use tokio::process::Command;
use tokio::select;
use tokio::time::sleep;
use tokio::sync::mpsc;
use crate::process_handler::Status;

async fn get_status(mut receiver: mpsc::Receiver<Status>) {
    while let Some(status) = receiver.recv().await {
        match status {
            Status::NotSpawned => println!("Status: NotSpawned"),
            Status::Starting => println!("Status: Starting"),
            Status::Running => println!("Status: Running"),
            Status::FailedToStart(err) => println!("Status: FailedToStart({})", err),
            Status::Exited(_) => {
                println!("Status: Exited");
                break;
            },
        }
    }
}

#[tokio::test]
async fn create_task() {
    let config = Program::try_from("/Users/basil/42/taskmaster/taskmaster.yaml")
        .expect("Failed to parse program");
    let handle = Routine::spawn(config).expect("failed to spawn tokio::task");
    let handle2 = tokio::spawn(get_status(handle.receiver));
    select! {
        _ = handle.join_handle => {},
        _ = sleep(Duration::from_secs(3)) => {},
    };
    handle2.await.expect("failed to join status handle");
}
