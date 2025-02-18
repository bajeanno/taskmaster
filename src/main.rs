//Author Basile Jeannot bajeanno 42Lyon Member

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

// impl Display for Task {
// 	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
// 		write!(f, "{}\t{}", self.id, self.name)
// 	}
// }

struct TaskServer {
	tasks: Vec<Task>,
}

impl TaskServer {
	fn new() -> Self {
		Self { tasks: Vec::new() }
	}

	fn run(&self) {
		loop {};
	}

	fn create_task(&mut self, task_name: &str){
		self.tasks.push(Task::new(self.tasks.len() as u32, task_name));
	}
}

fn main() {
    println!("Hello, task master!");
    let mut server = TaskServer::new();
    server.create_task("task 0");
    server.create_task("task 1");
    server.run();
}
