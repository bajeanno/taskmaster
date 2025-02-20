use core::time;
use std::{collections::HashMap, fmt::Display, thread};
mod parser;
use parser::{Parser, program::Program};

struct TaskServer {
    tasks: HashMap<String, Program>,
}

impl TaskServer {
    fn new(programs: HashMap<String, Program>) -> Self {
        Self { tasks: programs }
    }

    fn _run(&self) {
        loop {
            println!("Print out");
            eprintln!("Print err");
            thread::sleep(time::Duration::new(5, 0));
        }
    }

    fn list_tasks(&self) -> String {
        println!("{:<15}{:^10}{:10}", "program name", "pid","cmd");
        self.tasks
            .iter()
            .fold(String::new(), |acc, (_, value)| format!("{acc}{}\n", value))
    }
}

impl Display for TaskServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tasks: Vec<String> = self.tasks.iter().map(|(_, value)| format!("{value}")).collect();
        write!(f, "{}", tasks.join("\n"))
    }
}


// todo: fix yaml parsing
fn main() {
    println!("Hello, task master!");
    let tasks: HashMap<String, Program> = Parser::parse("taskmaster.yaml").unwrap_or_else(|err| {
        eprintln!("warning: {err}");
        panic!()
        // vec![]
    });
    let server = TaskServer::new(tasks);
    println!("{}", server.list_tasks());
    // unsafe {
    //     daemonize::Daemonize::new()
    //         .stdout("./server_output")
    //         .stderr("./server_output")
    //         .start()
    //         .expect("Failed to daemonize server")
    // }
    // server.run();
}
