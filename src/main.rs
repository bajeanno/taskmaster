use core::time;
use std::{fmt::Display, thread};
mod parser;
use parser::{program::Program, Parser};

struct TaskServer {
    tasks: Vec<Program>,
}

impl TaskServer {
    fn new(programs: Vec<Program>) -> Self {
        Self { tasks: programs }
    }

    fn run(&self) {
        loop {
            println!("Print out");
            eprintln!("Print err");
            thread::sleep(time::Duration::new(5, 0));
        }
    }

    fn list_tasks(&self) -> String {
        println!("{:<15}{:^50}{:10}", "program name", "cmd", "pids");
        self.tasks
            .iter()
            .fold(String::new(), |acc, value| format!("{acc}{}\n", value))
    }
}

impl Display for TaskServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tasks: Vec<String> = self.tasks.iter().map(|value| format!("{value}")).collect();
        write!(f, "{}", tasks.join("\n"))
    }
}

fn main() {
    println!("Hello, task master!");
    let tasks: Vec<Program> = Parser::parse("taskmaster.yaml").unwrap_or_else(|err| {
        eprintln!("Warning: {err}");
        Vec::new()
    });
    let server = TaskServer::new(tasks);
    println!("{}", server.list_tasks());
    unsafe {
        daemonize::Daemonize::new()
            .stdout("./server_output")
            .stderr("./server_output")
            .start()
            .expect("Failed to daemonize server")
    }
    server.run();
}
