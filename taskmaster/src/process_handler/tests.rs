use crate::process_handler::{Log, LogType, NominativeStatus, Routine, Status};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::{Mutex, mpsc::UnboundedReceiver};

fn test_log_paths(prefix: &str) -> (String, String) {
    let base = PathBuf::from("/tmp/").join("taskmaster_tests");
    std::fs::create_dir_all(&base).expect("failed to create local temp test directory");
    let unique = format!(
        "{}_{}_{}",
        prefix,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock is before UNIX_EPOCH")
            .as_nanos()
    );
    let stdout = base.join(format!("{unique}_stdout.txt"));
    let stderr = base.join(format!("{unique}_stderr.txt"));
    (
        stdout.to_string_lossy().to_string(),
        stderr.to_string_lossy().to_string(),
    )
}

async fn check_status(
    status_receiver: Arc<Mutex<UnboundedReceiver<NominativeStatus>>>,
    process_name: String,
) {
    let nominative_status = status_receiver.lock().await.recv().await.unwrap();
    assert_eq!(
        nominative_status.process_name, process_name,
        "process name doesn't match in nominative status, while expecting for Status::Starting"
    );
    assert!(
        matches!(nominative_status.status, Status::Starting),
        "expected status::starting, got {:?}",
        nominative_status.status
    );
    let nominative_status = status_receiver.lock().await.recv().await.unwrap();
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
    let nominative_status = status_receiver.lock().await.recv().await.unwrap();
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
                    assert_eq!(log.message, "taskmaster_test_task-0: Hello taskmaster!\n");
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
    use std::io::Cursor;

    use tokio::{fs::remove_file, sync::Mutex};

    use crate::config::Config;

    let (stdout_file, stderr_file) = test_log_paths("taskmaster_tests");
    let yaml_content = format!(
        r#"programs:
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
        stdout: {stdout}
        stderr: {stderr}
        clearenv: true
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
    let name = format!("{}-0", program.name());

    let routine_handle = Routine::spawn(program, status_sender, log_sender, name.clone(), 0);
    let log_checker_handle = tokio::spawn(check_realtime_output(log_receiver));
    let status_receiver = Arc::new(Mutex::new(status_receiver));
    let status_checker_handle =
        tokio::spawn(check_status(Arc::clone(&status_receiver), name.clone()));

    routine_handle.join().await;
    log_checker_handle
        .await
        .expect("failed to join status handle");
    status_checker_handle
        .await
        .expect("failed to join status handle");
    check_status_exited(Arc::clone(&status_receiver), &name).await;

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

    let (stdout_file, stderr_file) = test_log_paths("taskmaster_tests_interrupt");
    let yaml_content = format!(
        r#"programs:
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
        stdout: {stdout}
        stderr: {stderr}
        clearenv: true
        env:
            STARTED_BY: taskmaster
            ANSWER: 42"#,
        stdout = stdout_file,
        stderr = stderr_file
    );
    let config = Config::from_reader(Cursor::new(yaml_content))
        .expect("Parse error")
        .programs
        .into_iter()
        .next()
        .expect("Config hashmap is empty")
        .1;

    let (status_sender, status_receiver) = mpsc::unbounded_channel();
    let (log_sender, _) = mpsc::unbounded_channel();
    let name = format!("{}-0", config.name());
    let routine_handle = Routine::spawn(config, status_sender, log_sender, name.clone(), 0);
    let status_receiver: Arc<Mutex<UnboundedReceiver<NominativeStatus>>> =
        Arc::new(Mutex::new(status_receiver));
    let handle2 = tokio::spawn(check_status(Arc::clone(&status_receiver), name.clone()));

    handle2.await.expect("failed to join status handle"); // wait for running status to send stop signal
    routine_handle.stop_and_join().await;
    check_status_exited(Arc::clone(&status_receiver), &name).await; // check exited status after stop signal

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
