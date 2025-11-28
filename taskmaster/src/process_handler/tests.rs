use crate::parser::Program;
use crate::process_handler::{Log, Routine, Status};
use std::time::Duration;
use tokio::select;
use tokio::sync::mpsc;
use tokio::time::sleep;

#[cfg(test)]
async fn get_status(
    mut status_receiver: mpsc::Receiver<Status>,
    mut log_receiver: mpsc::Receiver<Log>,
) {
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
                match log {
                    Log::Stdout(log) => println!("Stdout: {log}"),
                    Log::Stderr(log) => println!("Stderr: {log}"),
                }
            },
            else => break,
        }
    }
}

#[cfg(test)]
#[tokio::test]
async fn create_task() {
    use tokio::{fs::{File, remove_file}, io::AsyncWriteExt};
    // create file /tmp/taskmaster_test.yaml

    let yaml_content = r#"
cmd: "bash -c \"echo Hello $STARTED_BY!\""
numprocs: 1
umask: 022
workingdir: /tmp
autostart: true
exitcodes:
  - 0
  - 2
startretries: 5
starttime: 0
stopsignal: TERM
stoptime: 10
stdout: /tmp/taskmaster_tests.stdout
stderr: /tmp/taskmaster_tests.stderr
env:
  STARTED_BY: taskmaster
  ANSWER: 42
"#;
    let test_file_name = "/tmp/taskmaster_test.yaml";
    let mut file = File::create(test_file_name).await.unwrap();
    file.write_all(yaml_content.as_bytes())
        .await
        .unwrap();

    let config = Program::try_from(test_file_name).expect("Failed to parse program");

    let routine_handle = Routine::spawn(config).expect("failed to spawn tokio::task");
    let handle2 = tokio::spawn(get_status(
        routine_handle.status_receiver,
        routine_handle.log_receiver,
    ));
    select! {
        _ = routine_handle.join_handle => {},
        _ = sleep(Duration::from_secs(3)) => {},
    };

    handle2.await.expect("failed to join status handle");
    remove_file("/tmp/taskmaster_tests.stdout")
        .await
        .inspect_err(|err| eprintln!("{err}"))
        .unwrap();
    remove_file("/tmp/taskmaster_tests.stderr")
        .await
        .inspect_err(|err| eprintln!("{err}"))
        .unwrap();
    remove_file(test_file_name)
        .await
        .inspect_err(|err| eprintln!("{err}"))
        .unwrap();
}
