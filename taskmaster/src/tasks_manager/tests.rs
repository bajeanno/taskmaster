#[tokio::test]
async fn task_manager() {
    use super::routine::Routine;
    use crate::tasks_manager::TaskManagerCommand;
    use crate::{convert_tasks_to_arc, get_tasks_from_config};
    use tokio::sync::oneshot;

    let filename = "/Users/basil/42/taskmaster/taskmaster.yaml";
    let tasks = get_tasks_from_config(filename);
    println!("{:?}", tasks);
    let tasks = convert_tasks_to_arc(tasks);
    let handle = Routine::spawn(tasks);
    let (sender, receiver) = oneshot::channel();
    println!(
        "{:?}",
        handle
            .send(TaskManagerCommand::ListTasks(sender))
            .await
            .unwrap()
    );
    receiver.await.expect("Receiver failed");
}

#[tokio::test]
async fn task_manager_stop() {
    use super::routine::Routine;
    use crate::tasks_manager::TaskManagerCommand;
    use crate::{convert_tasks_to_arc, get_tasks_from_config};

    let filename = "/Users/basil/42/taskmaster/taskmaster.yaml";
    let tasks = get_tasks_from_config(filename);
    let tasks = convert_tasks_to_arc(tasks);
    let handle = Routine::spawn(tasks);

    handle
        .send(TaskManagerCommand::StopTask {
            task_name: String::from("taskmaster_test_task"),
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn task_manager_start_already_started() {
    use super::routine::Routine;
    use crate::tasks_manager::TaskManagerCommand;
    use crate::{convert_tasks_to_arc, get_tasks_from_config};

    let filename = "/Users/basil/42/taskmaster/taskmaster.yaml";
    let tasks = get_tasks_from_config(filename);
    let tasks = convert_tasks_to_arc(tasks);
    let handle = Routine::spawn(tasks);

    handle
        .send(TaskManagerCommand::StartTask {
            task_name: String::from("taskmaster_test_task"),
        })
        .await
        .unwrap();
}
