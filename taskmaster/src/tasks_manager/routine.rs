use super::handle::Handle;
use crate::CommandReceiver;
use crate::{
    config::Program,
    process_handler::{
        self, LogReceiver, LogSender, ProcessStateChannel, RoutineSpawnError, Status,
        StatusReceiver, StatusSender,
    },
};
use std::{collections::HashMap, sync::Arc};
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::sync::{Mutex, oneshot};

#[derive(Debug, Error)]
enum StartTaskError {
    #[error("")]
    RoutineSpawnError(#[from] RoutineSpawnError),
}

pub struct Process {
    handle: Arc<process_handler::Handle>,
    status: Status,
}

#[allow(dead_code)]
pub struct Routine {
    tasks: Vec<Arc<Program>>,
    processes: Arc<Mutex<HashMap<String, Process>>>,
    command_receiver: CommandReceiver,
}

#[allow(dead_code)] //TODO: remove that
impl Routine {
    pub async fn spawn(tasks: Vec<Arc<Program>>) -> Handle {
        let (log_sender, log_receiver) = mpsc::unbounded_channel();
        let (status_sender, status_receiver) = mpsc::unbounded_channel();
        let (command_sender, command_receiver) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            Self {
                tasks,
                processes: Arc::new(Mutex::new(HashMap::new())),
                command_receiver,
            }
            .routine(log_sender, log_receiver, status_sender, status_receiver)
            .await;
        });

        Handle::new(command_sender)
    }

    async fn routine(
        mut self,
        log_sender: LogSender,
        log_receiver: LogReceiver,
        status_sender: StatusSender,
        status_receiver: StatusReceiver,
    ) {
        if let Err(_result) = self.start_tasks(status_sender, log_sender).await {
            self.stop_routines().await
        };
        let handle1 = tokio::spawn(Self::listen_for_logs(log_receiver));
        let handle2 = tokio::spawn(Self::listen_for_status(
            status_receiver,
            Arc::clone(&self.processes),
        ));

        self.event_listener().await;

        handle1.await.expect("listen_for_logs failed");
        handle2.await.expect("listen_for_status failed");
    }

    async fn start_tasks(
        &mut self,
        status_sender: StatusSender,
        log_sender: LogSender,
    ) -> Result<(), StartTaskError> {
        for task in self.tasks.clone().iter() {
            let num_procs = *task.num_procs();

            for id in 0..num_procs {
                let task_name = task.name().clone();
                let task_id = task_name.to_owned() + format!("_{}", id).as_str();
                let handle = self
                    .start_task(
                        Arc::clone(task),
                        status_sender.clone(),
                        log_sender.clone(),
                        task_id.clone(),
                    )
                    .await?;
                self.processes.lock().await.insert(
                    task_id,
                    Process {
                        handle: Arc::new(handle),
                        status: Status::Starting,
                    },
                );
            }
        }
        Ok(())
    }

    async fn start_task(
        &mut self,
        task: Arc<Program>,
        status_sender: StatusSender,
        log_sender: LogSender,
        task_id: String,
    ) -> Result<process_handler::Handle, StartTaskError> {
        let handle =
            process_handler::Routine::spawn(task, status_sender, log_sender, task_id).await?;
        Ok(handle)
    }

    async fn listen_for_status(
        mut status_receiver: StatusReceiver,
        process_hashmap: Arc<Mutex<HashMap<String, Process>>>,
    ) {
        while let Some(status) = status_receiver.recv().await {
            let handle = Arc::clone(
                &process_hashmap
                    .lock()
                    .await
                    .get(&status.process_name)
                    .unwrap()
                    .handle,
            );
            process_hashmap.lock().await.insert(
                status.process_name,
                Process {
                    handle,
                    status: status.status,
                },
            );
        }
    }

    /// logs are already written to log files, we only need to write them to the client if he asks for it
    async fn listen_for_logs(mut log_receiver: LogReceiver) {
        while let Some(_log) = log_receiver.recv().await {
            todo!() //TODO: do something with logs
        }
    }

    async fn event_listener(&mut self) {
        while let Some(command) = self.command_receiver.recv().await {
            match command {
                commands::ServerCommand::ListTasks => todo!("ListTasks (status command)"),
                commands::ServerCommand::Stop { process_name } => {
                    let mut map = self.processes.lock().await;
                    let mut process = map.get_mut(&process_name).unwrap_or_else(|| todo!());
                    Self::stop_routine(&mut process).await;
                }
                commands::ServerCommand::Restart { process_name } => {
                    todo!("Restart {}", process_name)
                }
                commands::ServerCommand::Start { process_name } => todo!("Start {}", process_name),
            }
        }
    }

    async fn stop_routines(&mut self) {
        for (_, process) in self.processes.lock().await.iter_mut() {
            match process.status {
                Status::Starting | Status::Running => Self::stop_routine(process).await,
                _ => {} //routine already stopped (crashed or exited)
            }
        }
    }

    /// Stops a routine by sending a kill command.
    ///
    /// # Warning
    /// This function should NOT be called before checking that the process has not exited.
    /// Ensure the process is still running before invoking this function.
    async fn stop_routine(entry: &mut Process) {
        let (s, r): ProcessStateChannel = oneshot::channel();
        entry
            .handle
            .kill_command_sender
            .send(s)
            .await
            // TODO don't expect
            .expect("Receiver was dropped");
        match r.await {
            Ok(response) => match response {
                process_handler::ProcessState::Running => {}
                process_handler::ProcessState::Stopped => todo!(),
            },
            Err(_) => {
                todo!()
            }
        }
    }
}
