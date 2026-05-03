use std::time::Duration;

use tokio::{task::futures::TaskLocalFuture, time::sleep};

use crate::NominativeStatus;

#[cfg(test)]
fn create_tasks() -> String {
    r#"programs:
    taskmaster_test_task:
        cmd: "cat"
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
        stdout: /tmp/taskmaster_tests.stdout
        stderr: /tmp/taskmaster_tests.stderr
        clearenv: true
        env:
            STARTED_BY: taskmaster
            ANSWER: 42"#
        .to_string()
}

#[tokio::test]
async fn task_manager_list_tasks() {
    use super::routine::Routine;
    use crate::config::Config;
    use crate::convert_tasks_to_arc;
    use crate::tasks_manager::TaskManagerCommand;
    use std::io::Cursor;
    use tokio::sync::oneshot;

    let yaml_content = create_tasks();
    let program = Config::from_reader(Cursor::new(yaml_content))
        .expect("Parse error")
        .programs
        .into_iter()
        .next()
        .expect("Config vector is empty");
    let tasks = vec![program];
    let tasks = convert_tasks_to_arc(tasks);
    let handle = Routine::spawn(tasks);
    let (sender, receiver) = oneshot::channel();
    handle
        .send(TaskManagerCommand::ListTasks(sender))
        .await
        .unwrap();
    receiver.await.expect("Receiver failed");
    handle.stop().await;
}

#[tokio::test]
async fn task_manager_stop() {
    use super::routine::Routine;
    use crate::config::Config;
    use crate::convert_tasks_to_arc;
    use crate::tasks_manager::TaskManagerCommand;
    use std::io::Cursor;

    let yaml_content = create_tasks();
    let config = Config::from_reader(Cursor::new(yaml_content))
        .expect("Parse error")
        .programs
        .into_iter()
        .next()
        .expect("Config vector is empty");
    let tasks = vec![config];
    let tasks = convert_tasks_to_arc(tasks);
    let handle = Routine::spawn(tasks);

    handle
        .send(TaskManagerCommand::StopTask {
            task_name: String::from("taskmaster_test_task"),
        })
        .await
        .unwrap();
    handle.stop().await;
}

#[tokio::test]
async fn task_manager_start_already_started() {
    use super::routine::Routine;
    use crate::config::Config;
    use crate::convert_tasks_to_arc;
    use crate::tasks_manager::TaskManagerCommand;
    use std::io::Cursor;

    let yaml_content = create_tasks();
    let config = Config::from_reader(Cursor::new(yaml_content))
        .expect("Parse error")
        .programs
        .into_iter()
        .next()
        .expect("Config vector is empty");
    let tasks = vec![config];
    let tasks = convert_tasks_to_arc(tasks);
    let handle = Routine::spawn(tasks);

    handle
        .send(TaskManagerCommand::StartTask {
            task_name: String::from("taskmaster_test_task"),
        })
        .await
        .unwrap();
    handle.stop().await;
}

#[tokio::test]
async fn task_manager_restart() {
    use super::routine::Routine;
    use crate::config::Config;
    use crate::convert_tasks_to_arc;
    use crate::tasks_manager::TaskManagerCommand;
    use std::io::Cursor;

    let yaml_content = create_tasks();
    let config = Config::from_reader(Cursor::new(yaml_content))
        .expect("Parse error")
        .programs
        .into_iter()
        .next()
        .expect("Config vector is empty");
    let tasks = vec![config];
    let arc_tasks = convert_tasks_to_arc(tasks);
    let handle = Routine::spawn(arc_tasks);

    handle
        .send(TaskManagerCommand::RestartTask {
            task_name: String::from("taskmaster_test_task"),
        })
        .await
        .unwrap();
    handle.stop().await;
}
