#[tokio::test]
async fn test_routine() {
    use super::routine::Routine;
    use crate::tasks_manager::TaskManagerCommand;
    use crate::{convert_tasks_to_arc, get_tasks_from_config};
    use tokio::sync::oneshot;
    let filename = "taskmaster.yaml";
    let tasks = get_tasks_from_config(filename);
    let tasks = convert_tasks_to_arc(tasks);

    let handle = Routine::spawn(tasks);
    let (sender, receiver) = oneshot::channel();
    handle
        .send(TaskManagerCommand::ListTasks(sender))
        .await
        .unwrap();
    receiver.await.expect("Receiver failed");
}
