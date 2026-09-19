use crate::process::ProcessId;
use crate::process_handler::log::LogType;
use crate::process_handler::{Log, NominativeStatus, Routine, Status};
use crate::tests::TestDir;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::{Mutex, mpsc::UnboundedReceiver};

async fn check_status(
    status_receiver: Arc<Mutex<UnboundedReceiver<NominativeStatus>>>,
    process_id: ProcessId,
) {
    let nominative_status = status_receiver.lock().await.recv().await.unwrap();
    assert_eq!(
        nominative_status.process_id, process_id,
        "process name doesn't match in nominative status, while expecting for Status::Starting"
    );
    assert!(
        matches!(nominative_status.status, Status::Starting),
        "expected status::starting, got {:?}",
        nominative_status.status
    );
    let nominative_status = status_receiver.lock().await.recv().await.unwrap();
    assert_eq!(
        nominative_status.process_id, process_id,
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
    process_id: &ProcessId,
) {
    let nominative_status = status_receiver.lock().await.recv().await.unwrap();
    assert_eq!(&nominative_status.process_id, process_id);
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
                    assert_eq!(log.message, "taskmaster_test_task-0: Hello taskmaster!\n");
                    assert_eq!(log.process_id.to_string(), "taskmaster_test_task-0");
                }
                LogType::Stderr => {
                    assert_eq!(log.message, "");
                    assert_eq!(log.process_id.to_string(), "taskmaster_test_task-0");
                }
            },
            None => break,
        }
    }
}

#[tokio::test]
async fn create_task() {
    use std::io::Cursor;

    use tokio::{fs::remove_file, sync::Mutex};

    use crate::config::Config;

    let base = TestDir::new("create_task");
    let (stdout_file, stderr_file) = (base.join("stdout.log"), base.join("stderr.log"));
    let yaml_content = format!(
        r#"programs:
    taskmaster_test_task:
        cmd: "bash -c \"echo Hello $STARTED_BY!\""
        num-procs: 1
        umask: 0o022
        working-dir: /tmp
        auto-start: true
        exit-codes:
        - 0
        - 2
        start-retries: 5
        start-time: 0
        stop-signal: SIGTERM
        stop-time: 10
        stdout: {stdout}
        stderr: {stderr}
        clear-env: true
        env:
            STARTED_BY: taskmaster
            ANSWER: 42"#,
        stdout = stdout_file,
        stderr = stderr_file
    );
    let program = Config::from_reader(Cursor::new(yaml_content))
        .expect("Parse error")
        .programs
        .into_iter()
        .next()
        .expect("Config vector is empty")
        .1;

    let (status_sender, status_receiver) = mpsc::unbounded_channel();
    let (log_sender, log_receiver) = mpsc::unbounded_channel();
    let process_id = ProcessId {
        task_name: program.name().clone(),
        id: 0,
    };

    let routine_handle = Routine::spawn(program, status_sender, log_sender, process_id.clone(), 0);
    let log_checker_handle = tokio::spawn(check_realtime_output(log_receiver));
    let status_receiver = Arc::new(Mutex::new(status_receiver));
    let status_checker_handle = tokio::spawn(check_status(
        Arc::clone(&status_receiver),
        process_id.clone(),
    ));

    routine_handle.join().await;
    log_checker_handle
        .await
        .expect("failed to join status handle");
    status_checker_handle
        .await
        .expect("failed to join status handle");
    check_status_exited(Arc::clone(&status_receiver), &process_id).await;

    let buffer_stdout = tokio::fs::read_to_string(&stdout_file)
        .await
        .expect("failed to read stdout file");
    let buffer_stderr = tokio::fs::read_to_string(&stderr_file)
        .await
        .expect("failed to read stderr file");

    remove_file(&stdout_file)
        .await
        .inspect_err(|err| eprintln!("{err}"))
        .unwrap();
    remove_file(&stderr_file)
        .await
        .inspect_err(|err| eprintln!("{err}"))
        .unwrap();

    assert_eq!(
        buffer_stdout.trim(),
        "taskmaster_test_task-0: Hello taskmaster!"
    );
    assert_eq!(buffer_stderr.trim(), "");
}

#[tokio::test]
async fn create_task_then_interrupt() {
    use crate::config::Config;
    use std::io::Cursor;
    use tokio::fs::remove_file;

    let base = TestDir::new("create_task").sub_dir("then_interrupt");
    let (stdout_file, stderr_file) = (base.join("stdout.log"), base.join("stderr.log"));
    let yaml_content = format!(
        r#"programs:
    taskmaster_test_task:
        cmd: "cat"
        num-procs: 1
        umask: 0o022
        working-dir: /tmp
        auto-start: true
        exit-codes:
        - 0
        - 2
        start-retries: 5
        start-time: 0
        stop-signal: SIGINT
        stop-time: 10
        stdout: {stdout}
        stderr: {stderr}
        clear-env: true
        env:
            STARTED_BY: taskmaster
            ANSWER: 42"#,
        stdout = stdout_file,
        stderr = stderr_file
    );
    let config = Config::from_reader(Cursor::new(yaml_content))
        .unwrap()
        .programs
        .into_iter()
        .next()
        .expect("Config hashmap is empty")
        .1;

    let (status_sender, status_receiver) = mpsc::unbounded_channel();
    let (log_sender, _) = mpsc::unbounded_channel();
    let process_id = ProcessId {
        task_name: config.name().clone(),
        id: 0,
    };
    let routine_handle = Routine::spawn(config, status_sender, log_sender, process_id.clone(), 0);
    let status_receiver: Arc<Mutex<UnboundedReceiver<NominativeStatus>>> =
        Arc::new(Mutex::new(status_receiver));
    let handle2 = tokio::spawn(check_status(
        Arc::clone(&status_receiver),
        process_id.clone(),
    ));

    handle2.await.expect("failed to join status handle"); // wait for running status to send stop signal
    routine_handle.stop_and_join().await;
    check_status_exited(Arc::clone(&status_receiver), &process_id).await; // check exited status after stop signal

    let buffer_stdout = tokio::fs::read_to_string(&stdout_file)
        .await
        .expect("failed to read stdout file");
    let buffer_stderr = tokio::fs::read_to_string(&stderr_file)
        .await
        .expect("failed to read stderr file");

    remove_file(&stdout_file)
        .await
        .inspect_err(|err| eprintln!("{err}"))
        .unwrap();
    remove_file(&stderr_file)
        .await
        .inspect_err(|err| eprintln!("{err}"))
        .unwrap();

    assert_eq!(buffer_stdout.trim(), "");
    assert_eq!(buffer_stderr.trim(), "");
}

#[tokio::test]
async fn send_reloaded_config_updates_running_routine_behavior() {
    use crate::config::Config;
    use std::io::Cursor;

    fn program_from_yaml(content: &str) -> Arc<crate::config::ProgramConfig> {
        Config::from_reader(Cursor::new(content.to_string()))
            .expect("failed to parse config")
            .programs
            .into_values()
            .next()
            .expect("config has no program")
    }

    let initial_yaml = r#"programs:
  reload_test:
    cmd: "sh -c 'sleep 0.1; exit 1'"
    num-procs: 1
    auto-start: true
    auto-restart: unexpected
    exit-codes: [1]"#;

    let reloaded_yaml = r#"programs:
  reload_test:
    cmd: "sh -c 'sleep 0.1; exit 1'"
    num-procs: 1
    auto-start: true
    auto-restart: unexpected
    exit-codes: [0]"#;

    let (status_sender, status_receiver) = mpsc::unbounded_channel();
    let (log_sender, _log_receiver) = mpsc::unbounded_channel();
    let name = ProcessId {
        task_name: "reload_test".to_string(),
        id: 0,
    };

    let routine_handle = Routine::spawn(
        program_from_yaml(initial_yaml),
        status_sender,
        log_sender,
        name.clone(),
        0,
    );
    let status_receiver = Arc::new(Mutex::new(status_receiver));

    check_status(Arc::clone(&status_receiver), name.clone()).await;

    routine_handle
        .send_reloaded_config(program_from_yaml(reloaded_yaml))
        .await;

    // The reloaded config makes exit code 1 unexpected, so once the program
    // exits the subroutine must restart it instead of giving up. Skip any
    // statuses (e.g. the extra Running emitted when the config is received)
    // until the program exits.
    let mut status = status_receiver.lock().await.recv().await.unwrap().status;
    while !matches!(status, Status::Exited(_)) {
        status = status_receiver.lock().await.recv().await.unwrap().status;
    }

    assert!(
        matches!(
            status_receiver.lock().await.recv().await.unwrap().status,
            Status::Starting
        ),
        "subroutine must restart the program using the reloaded config"
    );
    assert!(
        matches!(
            status_receiver.lock().await.recv().await.unwrap().status,
            Status::Running
        ),
        "restarted program must be running"
    );

    routine_handle.stop_and_join().await;
}

#[tokio::test]
async fn create_task_with_working_dir() {
    use crate::config::Config;
    use std::io::Cursor;
    use tokio::fs::remove_file;

    let base = TestDir::new("create_task").sub_dir("with_working_dir");
    let (stdout_file, stderr_file) = (base.join("stdout.log"), base.join("stderr.log"));
    let yaml_content = format!(
        r#"programs:
    taskmaster_test_task:
        cmd: "pwd"
        num-procs: 1
        umask: 0o022
        working-dir: /tmp
        auto-start: true
        exit-codes:
        - 0
        start-retries: 5
        start-time: 0
        stop-signal: SIGTERM
        stop-time: 10
        stdout: {stdout}
        stderr: {stderr}
        clear-env: true"#,
        stdout = stdout_file,
        stderr = stderr_file
    );

    let program = Config::from_reader(Cursor::new(yaml_content))
        .expect("Parse error")
        .programs
        .into_iter()
        .next()
        .expect("Config vector is empty")
        .1;

    let (status_sender, status_receiver) = mpsc::unbounded_channel();
    let (log_sender, _log_receiver) = mpsc::unbounded_channel();
    let process_id = ProcessId {
        task_name: program.name().clone(),
        id: 0,
    };

    let _routine_handle = Routine::spawn(program, status_sender, log_sender, process_id.clone(), 0);
    let status_receiver = Arc::new(Mutex::new(status_receiver));
    check_status(Arc::clone(&status_receiver), process_id.clone()).await;
    check_status_exited(Arc::clone(&status_receiver), &process_id).await;

    let buffer_stdout = tokio::fs::read_to_string(&stdout_file)
        .await
        .expect("failed to read stdout file");

    remove_file(&stdout_file)
        .await
        .inspect_err(|err| eprintln!("{err}"))
        .unwrap();
    remove_file(&stderr_file)
        .await
        .inspect_err(|err| eprintln!("{err}"))
        .unwrap();

    let expected = std::fs::canonicalize("/tmp")
        .unwrap()
        .to_string_lossy()
        .to_string();
    assert_eq!(
        buffer_stdout.trim(),
        format!("{}: {}", process_id, expected),
        "process should have run in /tmp directory"
    );
}
