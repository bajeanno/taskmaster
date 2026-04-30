use crate::NominativeStatus;
use crate::process_handler::{Log, LogType, Routine, Status};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::{Mutex, mpsc::UnboundedReceiver};
use tokio::time::sleep;

async fn check_status(status_receiver: Arc<Mutex<UnboundedReceiver<NominativeStatus>>>) {
    match status_receiver.lock().await.recv().await.unwrap().status {
        Status::Starting => {}
        other => panic!("Expected Status::Starting, got {other:?}"),
    }
    match status_receiver.lock().await.recv().await.unwrap().status {
        Status::Running => {}
        status => panic!("not expected {status:?}"),
    }
}

async fn check_status_exited(status_receiver: Arc<Mutex<UnboundedReceiver<NominativeStatus>>>) {
    match status_receiver.lock().await.recv().await.unwrap().status {
        Status::Exited(_) => {}
        status => panic!("not expected {status:?}"),
    }
}

async fn check_realtime_output(mut log_receiver: mpsc::UnboundedReceiver<Log>) {
    loop {
        match log_receiver.recv().await {
            Some(log) => match log.log_type {
                LogType::Stdout => {
                    assert_eq!(log.message, "Hello taskmaster!\n");
                    assert_eq!(log.process_name, "taskmaster_test_task-0");
                }
                LogType::Stderr => {
                    assert_eq!(log.message, "");
                    assert_eq!(log.process_name, "taskmaster_test_task-0");
                }
            },
            None => break,
        }
    }
}

#[tokio::test]
async fn create_task() {
    use std::{
        fs::File,
        io::{Cursor, Read},
    };

    use tokio::{
        fs::remove_file,
        sync::{Mutex, mpsc::UnboundedReceiver},
    };

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

    let (status_sender, status_receiver) = mpsc::unbounded_channel();
    let (log_sender, log_receiver) = mpsc::unbounded_channel();
    let name = config.name().to_owned() + "-0";
    let routine_handle = Routine::spawn(Arc::new(config), status_sender, log_sender, name)
        .await
        .expect("failed to spawn tokio::task");
    let log_checker_handle = tokio::spawn(check_realtime_output(log_receiver));
    let status_receiver: Arc<Mutex<UnboundedReceiver<NominativeStatus>>> =
        Arc::new(Mutex::new(status_receiver));
    let status_checker_handle = tokio::spawn(check_status(Arc::clone(&status_receiver)));

    routine_handle.join_handle.await.unwrap();
    log_checker_handle
        .await
        .expect("failed to join status handle");
    status_checker_handle
        .await
        .expect("failed to join status handle");
    check_status_exited(Arc::clone(&status_receiver)).await;

    sleep(Duration::from_secs(1)).await; // wait before reading the file as flushing can be a little too long on CI
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
async fn create_task_then_interrupt() {
    use crate::config::Config;
    use std::{
        fs::File,
        io::{Cursor, Read},
    };
    use tokio::{fs::remove_file, sync::oneshot};

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

    let (status_sender, status_receiver) = mpsc::unbounded_channel();
    let (log_sender, _) = mpsc::unbounded_channel();
    let name = config.name().to_owned() + "_0";
    let routine_handle = Routine::spawn(Arc::new(config), status_sender, log_sender, name)
        .await
        .expect("failed to spawn tokio::task");
    let status_receiver: Arc<Mutex<UnboundedReceiver<NominativeStatus>>> =
        Arc::new(Mutex::new(status_receiver));
    let handle2 = tokio::spawn(check_status(Arc::clone(&status_receiver)));

    handle2.await.expect("failed to join status handle"); // wait for running status to send stop signal
    let (s, r) = oneshot::channel();
    if let Err(e) = routine_handle.kill_command_sender.send(s).await {
        panic!("Failed to send stop signal: {:?}", e);
    }
    r.await.expect("error receiving process state");

    routine_handle.join_handle.await.unwrap();
    check_status_exited(Arc::clone(&status_receiver)).await; // check exited status after stop signal

    sleep(Duration::from_secs(1)).await; // wait before reading the file as flushing can be a little too long on CI
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
