use std::fmt::Display;

use mockall::automock;

use crate::parser::program::Program;

pub struct TaskManager {
    pub tasks: Vec<Program>,
}

impl TaskManager {
    pub fn new(tasks: Vec<Program>) -> Self {
        Self { tasks }
    }
}

#[automock]
pub trait TaskManagerTrait {
    fn list_tasks(&self) -> String;
}

impl TaskManagerTrait for TaskManager {
    fn list_tasks(&self) -> String {
        println!("{:<15}{:^50}{:10}", "program name", "cmd", "pids");
        self.tasks
            .iter()
            .fold(String::new(), |acc, value| format!("{acc}{}\n", value))
    }
}

impl Display for TaskManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tasks: Vec<String> = self.tasks.iter().map(|value| format!("{value}")).collect();
        write!(f, "{}", tasks.join("\n"))
    }
}
