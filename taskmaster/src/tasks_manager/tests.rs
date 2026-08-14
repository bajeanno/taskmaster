use super::routine::Routine;
use crate::config::Config;
use crate::tasks_manager::TaskManagerCommand;
use std::io::Cursor;
use tokio::sync::oneshot;

fn create_tasks() -> String {
    r#"programs:
    taskmaster_test_task:
        cmd: "sleep 100000"
        num-procs: 2
        umask: 022
        working-dir: /tmp
        auto-start: true
        exit-codes:
        - 0
        - 2
        start-retries: 5
        start-time: 0
        stop-signal: SIGTERM
        stop-time: 10
        stdout: /tmp/taskmaster_taskmanager_tests.stdout
        stderr: /tmp/taskmaster_taskmanager_tests.stderr
        clear-env: true
        env:
            STARTED_BY: taskmaster
            ANSWER: 42"#
        .to_string()
}

#[tokio::test]
async fn task_manager_list_tasks() {
    let yaml_content = create_tasks();
    let programs_configs = Config::from_reader(Cursor::new(yaml_content))
        .expect("Parse error")
        .programs;
    let handle = Routine::spawn(programs_configs);
    let (sender, receiver) = oneshot::channel();
    handle
        .send(TaskManagerCommand::ListProcesses(sender))
        .await
        .unwrap();
    receiver.await.expect("Receiver failed");
    handle.stop().await;
}

#[tokio::test]
async fn task_manager_stop() {
    let yaml_content = create_tasks();
    let programs_configs = Config::from_reader(Cursor::new(yaml_content))
        .expect("Parse error")
        .programs;
    let handle = Routine::spawn(programs_configs);

    handle
        .send(TaskManagerCommand::StopProgram {
            program_name: String::from("taskmaster_test_task"),
        })
        .await
        .unwrap();
    handle.stop().await;
}

#[tokio::test]
async fn task_manager_start_already_started() {
    let yaml_content = create_tasks();
    let programs_configs = Config::from_reader(Cursor::new(yaml_content))
        .expect("Parse error")
        .programs;
    let handle = Routine::spawn(programs_configs);

    handle
        .send(TaskManagerCommand::StartProgram {
            program_name: String::from("taskmaster_test_task"),
        })
        .await
        .unwrap();
    handle.stop().await;
}

#[tokio::test]
async fn task_manager_stop_then_start() {
    let yaml_content = create_tasks();
    let programs_configs = Config::from_reader(Cursor::new(yaml_content))
        .expect("Parse error")
        .programs;
    let handle = Routine::spawn(programs_configs);

    handle
        .send(TaskManagerCommand::StopProgram {
            program_name: String::from("taskmaster_test_task"),
        })
        .await
        .unwrap();
    handle
        .send(TaskManagerCommand::StartProgram {
            program_name: String::from("taskmaster_test_task"),
        })
        .await
        .unwrap();
    handle.stop().await;
}

#[tokio::test]
async fn task_manager_restart() {
    let yaml_content = create_tasks();
    let programs_configs = Config::from_reader(Cursor::new(yaml_content))
        .expect("Parse error")
        .programs;
    let handle = Routine::spawn(programs_configs);

    handle
        .send(TaskManagerCommand::RestartProgram {
            program_name: String::from("taskmaster_test_task"),
        })
        .await
        .unwrap();
    handle.stop().await;
}
