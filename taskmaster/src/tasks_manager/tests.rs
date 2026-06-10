use super::routine::Routine;
use crate::config_state::ConfigState;
use crate::tasks_manager::TaskManagerCommand;
use tokio::sync::oneshot;

fn create_tasks_yaml_content() -> String {
    r#"programs:
    taskmaster_test_task:
        cmd: "sleep 100000"
        numprocs: 2
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
        stdout: /tmp/taskmaster_taskmanager_tests.stdout
        stderr: /tmp/taskmaster_taskmanager_tests.stderr
        clearenv: true
        env:
            STARTED_BY: taskmaster
            ANSWER: 42"#
        .to_string()
}

#[tokio::test]
async fn task_manager_list_tasks() {
    let handle = Routine::spawn(ConfigState::from_content(create_tasks_yaml_content()));
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
    let handle = Routine::spawn(ConfigState::from_content(create_tasks_yaml_content()));
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
    let handle = Routine::spawn(ConfigState::from_content(create_tasks_yaml_content()));

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
    let handle = Routine::spawn(ConfigState::from_content(create_tasks_yaml_content()));
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
    let handle = Routine::spawn(ConfigState::from_content(create_tasks_yaml_content()));

    handle
        .send(TaskManagerCommand::RestartProgram {
            program_name: String::from("taskmaster_test_task"),
        })
        .await
        .unwrap();
    handle.stop().await;
}
