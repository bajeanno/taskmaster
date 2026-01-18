use crate::parser::program::Program;
use crate::process_handler::{Log, LogType, Routine, Status};
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
                    Status::FailedToStart{ error_message, exit_code: _ } => println!("Status: FailedToStart: {error_message}"),
                    Status::Exited(_) => {
                        println!("Status: Exited");
                        break;
                    }
                }
            },

            Some(log) = log_receiver.recv() => {
                match log.log_type {
                    LogType::Stdout => {
                        assert_eq!(log.message, "Hello taskmaster!\n");
                        assert_eq!(log.program_name, "taskmaster_test_task");
                    },
                    LogType::Stderr => {
                        assert_eq!(log.message, "");
                        assert_eq!(log.program_name, "taskmaster_test_task");
                    },
                }
            },
            else => break,
        }
    }
}

#[tokio::test]
#[cfg(test)]
async fn create_task() {
    use std::{fs::File, io::Read};

    use tokio::fs::remove_file;

    let yaml_content = r#"cmd: "bash -c \"echo Hello $STARTED_BY!\""
name: "taskmaster_test_task"
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
  ANSWER: 42"#;
    let config = Program::try_from(yaml_content).expect("Failed to parse program");

    let routine_handle = Routine::spawn(config)
        .await
        .expect("failed to spawn tokio::task");
    let handle2 = tokio::spawn(get_status(
        routine_handle.status_receiver,
        routine_handle.log_receiver,
    ));

    routine_handle.join_handle.await.unwrap();
    handle2.await.expect("failed to join status handle");

    let stdout_file = "/tmp/taskmaster_tests.stdout";
    let stderr_file = "/tmp/taskmaster_tests.stderr";

    let mut file = File::open(stdout_file).expect("failed to open stdout file");
    let mut buffer: Vec<u8> = Vec::new();
    file.read_to_end(&mut buffer)
        .expect("failed to read stdout file");
    {
        let buffer = String::from_utf8(buffer).expect("failed to convert stdout to string");
        assert_eq!(buffer.trim(), "Hello taskmaster!");
    }

    file = File::open(stderr_file).expect("failed to open stderr file");
    buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .expect("failed to read stderr file");
    {
        let buffer = String::from_utf8(buffer).expect("failed to convert stderr to string");
        assert_eq!(buffer.trim(), "");
    }

    remove_file("/tmp/taskmaster_tests.stdout")
        .await
        .inspect_err(|err| eprintln!("{err}"))
        .unwrap();
    remove_file("/tmp/taskmaster_tests.stderr")
        .await
        .inspect_err(|err| eprintln!("{err}"))
        .unwrap();
}
