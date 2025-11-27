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

#[cfg(test)]
async fn get_status(mut status_receiver: mpsc::Receiver<Status>, mut log_receiver: mpsc::Receiver<String>) {
    loop {
        select! {
            Some(status) = status_receiver.recv() => {
                match status {
                    Status::NotSpawned => println!("Status: NotSpawned"),
                    Status::Starting => println!("Status: Starting"),
                    Status::Running => println!("Status: Running"),
                    Status::FailedToStart(err) => println!("Status: FailedToStart({})", err),
                    Status::Exited(_) => {
                        println!("Status: Exited");
                        break;
                    }
                }
            },
            Some(log) = log_receiver.recv() => {
                println!("Log: {}", log);
            },
            else => break,
        }
    }
}

#[cfg(test)]
#[tokio::test]
async fn create_task() {
    let config = Program::try_from("/Users/basil/42/taskmaster/taskmaster.yaml")
        .expect("Failed to parse program");
    let routine_handle = Routine::spawn(config).expect("failed to spawn tokio::task");
    let handle2 = tokio::spawn(get_status(routine_handle.status_receiver, routine_handle.log_receiver));
    select! {
        _ = routine_handle.join_handle => {},
        _ = sleep(Duration::from_secs(3)) => {},
    };
    handle2.await.expect("failed to join status handle");
}
