use std::fs::OpenOptions;
use std::io::Write;

use super::routine::Routine;
use crate::config_state::ConfigState;
use crate::tasks_manager::TaskManagerCommand;
use tokio::sync::oneshot;

fn create_tasks_yaml_content() -> String {
    r#"programs:
    taskmaster_test_task:
        cmd: "sleep 30"
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
        clearenv: true
        env:
            STARTED_BY: taskmaster
            ANSWER: 42"#
        .to_string()
}

fn create_tasks_yaml_content_reload() -> String {
    r#"programs:
    reload:
        cmd: "sleep 30"
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
        clearenv: true
        env:
            STARTED_BY: taskmaster
            ANSWER: 42"#
        .to_string()
}

fn create_tasks_alternate_yaml_content_minus_1_proc() -> String {
    r#"programs:
    reload:
        cmd: "sleep 30"
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
        clearenv: true
        env:
            STARTED_BY: taskmaster
            ANSWER: 42"#
        .to_string()
}

fn create_tasks_alternate_yaml_content_plus_1_proc() -> String {
    r#"programs:
    reload:
        cmd: "sleep 30"
        numprocs: 3
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

#[tokio::test]
async fn task_manager_reload_minus_1_proc() {
    let handle = Routine::spawn(ConfigState::from_content(create_tasks_yaml_content_reload()));
    let new_content = create_tasks_alternate_yaml_content_plus_1_proc();
    let new_file = "/tmp/taskmaster_tests/taskmaster_task_manager_reload.yaml".to_string();
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(new_file.as_str())
        .expect("failed to create new taskmaster config file");
    file.write_all(new_content.as_bytes())
        .expect("failed to write new taskmaster config file");
    {
        let (s, r) = oneshot::channel();
        handle
            .send(TaskManagerCommand::ListProcesses(s))
            .await
            .unwrap();
        println!(
            "running tasks before reload command: {:?}",
            r.await.unwrap()
        );
    }
    handle
        .send(TaskManagerCommand::Reload {
            config_file_name: "/tmp/taskmaster_tests/taskmaster_task_manager_reload.yaml"
                .to_string(),
        })
        .await
        .unwrap();
    {
        let (s, r) = oneshot::channel();
        handle
            .send(TaskManagerCommand::ListProcesses(s))
            .await
            .unwrap();
        println!("running tasks after reload command: {:?}", r.await.unwrap());
    }
    handle.stop().await;
}

#[tokio::test]
async fn task_manager_reload_plus_1_proc() {
    let handle = Routine::spawn(ConfigState::from_content(create_tasks_yaml_content_reload()));
    let new_content = create_tasks_alternate_yaml_content_minus_1_proc();
    let new_file = "/tmp/taskmaster_tests/taskmaster_task_manager_reload.yaml".to_string();
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(new_file.as_str())
        .expect("failed to create new taskmaster config file");
    file.write_all(new_content.as_bytes())
        .expect("failed to write new taskmaster config file");
    {
        let (s, r) = oneshot::channel();
        handle
            .send(TaskManagerCommand::ListProcesses(s))
            .await
            .unwrap();
        println!(
            "running tasks before reload command: {:?}",
            r.await.unwrap()
        );
    }
    handle
        .send(TaskManagerCommand::Reload {
            config_file_name: "/tmp/taskmaster_tests/taskmaster_task_manager_reload.yaml"
                .to_string(),
        })
        .await
        .unwrap();
    {
        let (s, r) = oneshot::channel();
        handle
            .send(TaskManagerCommand::ListProcesses(s))
            .await
            .unwrap();
        println!("running tasks after reload command: {:?}", r.await.unwrap());
    }
    handle.stop().await;
}

#[tokio::test]
async fn task_manager_reload_keeps_unchanged_program() {
    let initial_content = r#"programs:
    keep:
        cmd: "sleep 30"
        autostart: true
    remove:
        cmd: "sleep 30"
        autostart: true"#;

    let new_content = r#"programs:
    keep:
        cmd: "sleep 30"
        autostart: true
    add:
        cmd: "sleep 30"
        autostart: true"#;

    let handle = Routine::spawn(ConfigState::from_content(initial_content.to_string()));

    let new_file = "/tmp/taskmaster_tests/taskmaster_reload_keeps_unchanged.yaml".to_string();
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(new_file.as_str())
        .expect("failed to create new taskmaster config file");
    file.write_all(new_content.as_bytes())
        .expect("failed to write new taskmaster config file");

    handle
        .send(TaskManagerCommand::Reload {
            config_file_name: new_file,
        })
        .await
        .unwrap();

    let (sender, receiver) = oneshot::channel();
    handle
        .send(TaskManagerCommand::ListProcesses(sender))
        .await
        .unwrap();
    let processes = receiver.await.expect("Receiver failed");

    let process_names: Vec<String> = processes
        .iter()
        .flatten()
        .map(|process| process.process_name.clone())
        .collect();

    assert!(
        process_names.contains(&"keep-0".to_string()),
        "unchanged program must survive a reload"
    );
    assert!(
        process_names.contains(&"add-0".to_string()),
        "new program must be started on reload"
    );
    assert!(
        !process_names.contains(&"remove-0".to_string()),
        "removed program must be stopped on reload"
    );

    handle.stop().await;
}
