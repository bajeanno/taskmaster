use crate::process_handler::{Log, LogType, NominativeStatus, Routine, Status};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::{Mutex, mpsc::UnboundedReceiver};

async fn check_status(
    status_receiver: Arc<Mutex<UnboundedReceiver<NominativeStatus>>>,
    process_name: String,
) {
    let nominative_status = status_receiver.lock().await.recv().await.unwrap().clone();
    assert_eq!(
        nominative_status.process_name, process_name,
        "process name doesn't match in nominative status, while expecting for Status::Starting"
    );
    assert!(
        matches!(nominative_status.status, Status::Starting),
        "expected status::starting, got {:?}",
        nominative_status.status
    );
    let nominative_status = status_receiver.lock().await.recv().await.unwrap().clone();
    assert_eq!(
        nominative_status.process_name, process_name,
        "process name doesn't match in nominative status, while expecting for Status::Running"
    );
    assert!(
        matches!(nominative_status.status, Status::Running),
        "expected Status::Running, got {:?}",
        nominative_status.status
    );
}

async fn check_status_exited(
    status_receiver: Arc<Mutex<UnboundedReceiver<NominativeStatus>>>,
    process_name: &str,
) {
    let nominative_status = status_receiver.lock().await.recv().await.unwrap().clone();
    assert_eq!(nominative_status.process_name, process_name);
    assert!(
        matches!(nominative_status.status, Status::Exited(_)),
        "not expected {:?}",
        nominative_status.status
    );
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

    use tokio::{fs::remove_file, sync::Mutex};

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
    let program = Config::from_reader(Cursor::new(yaml_content))
        .expect("Parse error")
        .programs
        .into_iter()
        .next()
        .expect("Config vector is empty");

    let (status_sender, status_receiver) = mpsc::unbounded_channel();
    let (log_sender, log_receiver) = mpsc::unbounded_channel();
    let name = format!("{}-0", program.name());
    let routine_handle = Routine::spawn(Arc::new(program), status_sender, log_sender, name.clone())
        .await
        .expect("failed to spawn tokio::task");
    let log_checker_handle = tokio::spawn(check_realtime_output(log_receiver));
    let status_receiver = Arc::new(Mutex::new(status_receiver));
    let status_checker_handle =
        tokio::spawn(check_status(Arc::clone(&status_receiver), name.clone()));

    routine_handle.wait_for_routine_to_finish().await;
    log_checker_handle
        .await
        .expect("failed to join status handle");
    status_checker_handle
        .await
        .expect("failed to join status handle");
    check_status_exited(Arc::clone(&status_receiver), &name).await;

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
    use tokio::fs::remove_file;

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
    let name = format!("{}-0", config.name());
    let routine_handle = Routine::spawn(Arc::new(config), status_sender, log_sender, name.clone())
        .await
        .expect("failed to spawn tokio::task");
    let status_receiver: Arc<Mutex<UnboundedReceiver<NominativeStatus>>> =
        Arc::new(Mutex::new(status_receiver));
    let handle2 = tokio::spawn(check_status(Arc::clone(&status_receiver), name.clone()));

    handle2.await.expect("failed to join status handle"); // wait for running status to send stop signal
    routine_handle.join().await;
    check_status_exited(Arc::clone(&status_receiver), &name).await; // check exited status after stop signal

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
