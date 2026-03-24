use crate::process_handler::{Log, LogType, Routine, Status};
use tokio::sync::mpsc;

#[cfg(test)]
async fn check_status(mut status_receiver: mpsc::UnboundedReceiver<Status>) {
    match status_receiver.recv().await.unwrap() {
        Status::Starting => {}
        other => panic!("Expected Status::Starting, got {}", other),
    }
    match status_receiver.recv().await.unwrap() {
        Status::Running => {}
        _ => panic!(),
    }
    match status_receiver.recv().await.unwrap() {
        Status::Exited(_) => {}
        _ => panic!(),
    }
}

#[cfg(test)]
async fn check_realtime_output(mut log_receiver: mpsc::UnboundedReceiver<Log>) {
    loop {
        match log_receiver.recv().await {
            Some(log) => match log.log_type {
                LogType::Stdout => {
                    assert_eq!(log.message, "Hello taskmaster!\n");
                    assert_eq!(log.program_name, "taskmaster_test_task");
                }
                LogType::Stderr => {
                    assert_eq!(log.message, "");
                    assert_eq!(log.program_name, "taskmaster_test_task");
                }
            },
            None => break,
        }
    }
}

#[tokio::test]
#[cfg(test)]
async fn create_task() {
    use std::{
        fs::File,
        io::{Cursor, Read},
    };

    use tokio::fs::remove_file;

    use crate::config::Config;

    let yaml_content = r#"programs:
    taskmaster_test_task:
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
        stopsignal: SIGTERM
        stoptime: 10
        stdout: /tmp/taskmaster_tests.stdout
        stderr: /tmp/taskmaster_tests.stderr
        clearenv: true
        env:
            STARTED_BY: taskmaster
            ANSWER: 42"#;
    let config = Config::from_reader(Cursor::new(yaml_content))
        .expect("Parse error")
        .programs
        .into_iter()
        .next()
        .expect("Config vector is empty");

    let routine_handle = Routine::spawn(config)
        .await
        .expect("failed to spawn tokio::task");
    let log_checker_handle = tokio::spawn(check_realtime_output(routine_handle.log_receiver));
    let status_checker_handle = tokio::spawn(check_status(routine_handle.status_receiver));

    routine_handle.join_handle.await.unwrap();
    log_checker_handle
        .await
        .expect("failed to join status handle");
    status_checker_handle
        .await
        .expect("failed to join status handle");

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
    remove_file(stderr_file)
        .await
        .inspect_err(|err| eprintln!("{err}"))
        .unwrap();
}

#[tokio::test]
#[cfg(test)]
async fn create_task_then_interrupt() {
    use crate::config::Config;
    use std::{
        fs::File,
        io::{Cursor, Read},
        time::Duration,
    };
    use tokio::{fs::remove_file, sync::oneshot, time::sleep};

    let yaml_content = r#"programs:
    taskmaster_test_task:
        cmd: "cat"
        numprocs: 1
        umask: 022
        workingdir: /tmp
        autostart: true
        exitcodes:
        - 0
        - 2
        startretries: 5
        starttime: 0
        stopsignal: SIGINT
        stoptime: 10
        stdout: /tmp/taskmaster_tests_interrupt.stdout
        stderr: /tmp/taskmaster_tests_interrupt.stderr
        clearenv: true
        env:
            STARTED_BY: taskmaster
            ANSWER: 42"#;
    let config = Config::from_reader(Cursor::new(yaml_content))
        .expect("Parse error")
        .programs
        .into_iter()
        .next()
        .expect("Config vector is empty");

    let routine_handle = Routine::spawn(config)
        .await
        .expect("failed to spawn tokio::task");
    let handle2 = tokio::spawn(check_status(routine_handle.status_receiver));

    sleep(Duration::from_secs(1)).await;

    let (s, r) = oneshot::channel();
    if let Err(e) = routine_handle.kill_command_sender.send(s).await {
        panic!("Failed to send stop signal: {:?}", e);
    }
    r.await.expect("error receiving process state");

    routine_handle.join_handle.await.unwrap();
    handle2.await.expect("failed to join status handle");

    let stdout_file = "/tmp/taskmaster_tests_interrupt.stdout";
    let stderr_file = "/tmp/taskmaster_tests_interrupt.stderr";

    let mut file = File::open(stdout_file).expect("failed to open stdout file");
    let mut buffer: Vec<u8> = Vec::new();
    file.read_to_end(&mut buffer)
        .expect("failed to read stdout file");
    {
        let buffer = String::from_utf8(buffer).expect("failed to convert stdout to string");
        assert_eq!(buffer.trim(), "");
    }

    file = File::open(stderr_file).expect("failed to open stderr file");
    buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .expect("failed to read stderr file");
    {
        let buffer = String::from_utf8(buffer).expect("failed to convert stderr to string");
        assert_eq!(buffer.trim(), "");
    }

    remove_file(stdout_file)
        .await
        .inspect_err(|err| eprintln!("{err}"))
        .unwrap();
    remove_file(stderr_file)
        .await
        .inspect_err(|err| eprintln!("{err}"))
        .unwrap();
}
