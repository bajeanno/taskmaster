#[allow(unused)]
use crate::Program;
#[allow(unused)]
use crate::process_handler::Routine;
#[allow(unused)]
use tokio::process::Command;
use tokio::task::JoinHandle;

#[tokio::test]
async fn create_task() {
    let mut cmd: Command = Command::new("echo");
    cmd.args(&["Hello, world!"]);
    let program = Program::try_from("/Users/basil/42/taskmaster/taskmaster.yaml")
        .expect("Failed to parse program");
    let join_handle: JoinHandle<()> = Routine::spawn(cmd, program)
        .expect("failed to spawn tokio::task")
        .join_handle;
    join_handle.await.expect("Task panicked");
    ()
}
