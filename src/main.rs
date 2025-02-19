use core::time;
use std::{fmt::Display, thread};

#[derive(Debug)]
struct Task {
    id: u32,
    name: String,
}

impl Task {
    fn new(task_id: u32, name: &str) -> Self {
        Self {
            id: task_id,
            name: String::from(name),
        }
    }
}

struct TaskServer {
    tasks: Vec<Task>,
}

impl TaskServer {
    fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    fn run(&self) {
        loop {
            println!("Print out");
            eprintln!("Print err");
            thread::sleep(time::Duration::new(5, 0));
        }
    }

    fn create_task(&mut self, task_name: &str) {
        self.tasks
            .push(Task::new(self.tasks.len() as u32, task_name));
    }
}

impl Display for TaskServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tasks: Vec<String> = self
            .tasks
            .iter()
            .map(|task| format!("{}\t{}", task.id, task.name))
            .collect();
        write!(f, "{}", tasks.join("\n"))
    }
}

fn main() {
    println!("Hello, task master!");
    unsafe {
        daemonize::Daemonize::new()
            .stdout("./server_output")
            .stderr("./server_output")
            .start()
            .expect("Failed to daemonize server")
    }

    let runtime = tokio::runtime::Runtime::new();

    let mut server = TaskServer::new();
    server.create_task("task 0");
    server.create_task("task 1");
    println!("{}", server);
    server.run();
}
