use std::{fmt::Display, sync::Arc};

use crate::{
    config::ProgramConfig,
    process_handler::{NominativeStatus, Status},
};

#[allow(dead_code)]
#[derive(Debug)]
pub struct ProcessList {
    pub(super) list: Vec<StatusList>,
}

#[allow(dead_code)]
impl ProcessList {
    pub fn new() -> Self {
        Self { list: Vec::new() }
    }

    pub fn push(&mut self, program_config: &Arc<ProgramConfig>, list: Vec<NominativeStatus>) {
        let list = StatusList {
            list,
            task_name: program_config.name().clone(),
            command: program_config.cmd().to_string(),
        };
        self.list.push(list);
    }
}

impl Display for ProcessList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.list.iter().fold(String::new(), |acc, list| {
                format!(
                    "{acc}{} ({}):\n\t{}\n\n",
                    list.task_name, list.command, list
                )
            })
        )
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub(super) struct StatusList {
    pub(super) list: Vec<NominativeStatus>,
    task_name: String,
    command: String,
}

impl StatusList {
    fn collect_statuses(&self) -> String {
        let mut not_running_count = 0;
        let mut routine_starting_count = 0;
        let mut starting_count = 0;
        let mut running_count = 0;
        let mut failed_to_start_count = 0;
        let mut startup_error_count = 0;
        let mut exited_count = 0;
        let mut failed_to_spawn_count = 0;
        let mut not_restarting_count = 0;

        self.list.iter().for_each(|nstatus| match nstatus.status {
            Status::NotRunning => not_running_count += 1,
            Status::RoutineStarting => routine_starting_count += 1,
            Status::Starting => starting_count += 1,
            Status::Running => running_count += 1,
            Status::FailedToStartProcess(_) => failed_to_start_count += 1,
            Status::ErrorDuringStartup(_) => startup_error_count += 1,
            Status::Exited(_) => exited_count += 1,
            Status::FailedToSpawnRoutine(_) => failed_to_spawn_count += 1,
            Status::NotRestarting { instance_id: _ } => not_restarting_count += 1,
        });

        let mut statuses = String::new();
        statuses = append_status(statuses, "NotRunning", not_running_count);
        statuses = append_status(statuses, "RoutineStarting", routine_starting_count);
        statuses = append_status(statuses, "Starting", starting_count);
        statuses = append_status(statuses, "Running", running_count);
        statuses = append_status(statuses, "FailedToStartProcess", failed_to_start_count);
        statuses = append_status(statuses, "StartupError", startup_error_count);
        statuses = append_status(statuses, "Exited", exited_count);
        statuses = append_status(statuses, "FailedToSpawn", failed_to_spawn_count);
        statuses = append_status(statuses, "NotRestarting", not_restarting_count);
        statuses
    }
}

fn append_status(mut statuses: String, status_name: &str, status_count: usize) -> String {
    if status_count != 0 {
        if !statuses.is_empty() {
            statuses += ", ";
        }
        statuses += status_name;
        statuses += " -> ";
        statuses += status_count.to_string().as_str();
    }
    statuses
}

impl Display for StatusList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.collect_statuses())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        process::ProcessId,
        process_handler::{NominativeStatus, Status},
        tasks_manager::process_list::{ProcessList, StatusList},
    };

    fn create_tasks_yaml_content() -> String {
        r#"programs:
    nginx:
        cmd: "/usr/bin/nginx -conf /etc/nginx.conf"
        num-procs: 4
    postgres:
        cmd: "/usr/bin/postgres"
        num-procs: 4"#
            .to_string()
    }

    #[test]
    fn status_list_print() {
        let vec = vec![
            NominativeStatus {
                process_id: ProcessId {
                    task_name: "nginix".to_string(),
                    id: 0,
                },
                status: Status::NotRunning,
            },
            NominativeStatus {
                process_id: ProcessId {
                    task_name: "nginix".to_string(),
                    id: 0,
                },
                status: Status::NotRunning,
            },
            NominativeStatus {
                process_id: ProcessId {
                    task_name: "nginix".to_string(),
                    id: 0,
                },
                status: Status::NotRunning,
            },
            NominativeStatus {
                process_id: ProcessId {
                    task_name: "nginix".to_string(),
                    id: 0,
                },
                status: Status::NotRunning,
            },
        ];
        let list = StatusList {
            list: vec,
            task_name: "nginx".to_string(),
            command: "/bin/nginx".to_string(),
        };
        assert_eq!(list.to_string(), "NotRunning -> 4".to_string())
    }

    #[test]
    fn process_list_print() {
        let list = vec![
            NominativeStatus {
                process_id: ProcessId {
                    task_name: "nginix".to_string(),
                    id: 0,
                },
                status: Status::Running,
            },
            NominativeStatus {
                process_id: ProcessId {
                    task_name: "nginix".to_string(),
                    id: 1,
                },
                status: Status::Starting,
            },
            NominativeStatus {
                process_id: ProcessId {
                    task_name: "nginix".to_string(),
                    id: 2,
                },
                status: Status::Starting,
            },
            NominativeStatus {
                process_id: ProcessId {
                    task_name: "nginix".to_string(),
                    id: 3,
                },
                status: Status::RoutineStarting,
            },
        ];
        let list2 = vec![
            NominativeStatus {
                process_id: ProcessId {
                    task_name: "postgres".to_string(),
                    id: 0,
                },
                status: Status::Running,
            },
            NominativeStatus {
                process_id: ProcessId {
                    task_name: "postgres".to_string(),
                    id: 1,
                },
                status: Status::Running,
            },
            NominativeStatus {
                process_id: ProcessId {
                    task_name: "postgres".to_string(),
                    id: 2,
                },
                status: Status::Running,
            },
            NominativeStatus {
                process_id: ProcessId {
                    task_name: "postgres".to_string(),
                    id: 3,
                },
                status: Status::Starting,
            },
        ];
        let content = create_tasks_yaml_content();
        let config = crate::config::Config::from_reader(std::io::Cursor::new(content)).unwrap();
        let mut process_list = ProcessList::new();
        process_list.push(&config.programs.get("nginx").unwrap(), list);
        process_list.push(&config.programs.get("postgres").unwrap(), list2);
        println!("{}", process_list.to_string());
        assert_eq!(
            process_list.to_string(),
            "nginx (/usr/bin/nginx [\"-conf\", \"/etc/nginx.conf\"]):\n\tRoutineStarting -> 1, Starting -> 2, Running -> 1\n\npostgres (/usr/bin/postgres):\n\tStarting -> 1, Running -> 3\n\n"
        )
    }
}
