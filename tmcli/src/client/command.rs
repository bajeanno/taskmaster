use std::fmt::Display;

use crate::client::session::Session;

#[derive(Debug)]
pub enum Command {
    ListTasks,
    StartProgram(String),
    StopProgram(String),
    RestartProgram(String),
    ReloadConfigFile,
    StopDaemon,
}

impl Command {
    pub async fn send(&self, _conn: &Session) -> Result<(), PlaceHolderError> {
        match self {
            Command::ListTasks => {
                list_tasks()
                    .call(_conn)
                    .await?
                    .into_iter()
                    .for_each(|item| println!("\t{item}"));
            }
            Command::StartProgram(task) => {
                start(task.to_owned()).call(_conn).await?.unwrap(); //TODO: check value at unwrap
            }
            Command::StopProgram(task) => {
                stop(task.to_owned()).call(_conn).await?.unwrap(); //TODO: check value at unwrap
            }
            Command::RestartProgram(task) => {
                restart(task.to_owned()).call(_conn).await?.unwrap(); //TODO: check value at unwrap
            }
            Command::ReloadConfigFile => {
                reload().call(_conn).await?.unwrap(); //TODO: check value at unwrap
            }
            Command::StopDaemon => {
                shutdown().call(_conn).await?.unwrap(); //TODO: check value at unwrap
            }
        }
        Ok(())
    }
}

struct PlaceHolder<T> {
    return_value: T,
}

#[derive(Debug)]
pub struct PlaceHolderError;

impl Display for PlaceHolderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PlaceHolderError")
    }
}

impl<T> PlaceHolder<T> {
    fn __new(return_value: T) -> Self {
        Self { return_value }
    }

    async fn call(self, _conn: &Session) -> Result<T, PlaceHolderError> {
        Ok(self.return_value)
    }
}

// #[rpc_genie::rpc]
fn list_tasks() -> PlaceHolder<Vec<String>> {
    PlaceHolder::__new(vec!["nginx".to_string(), "transcendence".to_string()])
}

// #[rpc_genie::rpc]
fn start(_task: String) -> PlaceHolder<Result<(), PlaceHolderError>> {
    PlaceHolder::__new(Ok(()))
}

// #[rpc_genie::rpc]
fn stop(_task: String) -> PlaceHolder<Result<(), PlaceHolderError>> {
    PlaceHolder::__new(Ok(()))
}

// #[rpc_genie::rpc]
fn restart(_task: String) -> PlaceHolder<Result<(), PlaceHolderError>> {
    PlaceHolder::__new(Ok(()))
}

// #[rpc_genie::rpc]
fn reload() -> PlaceHolder<Result<(), PlaceHolderError>> {
    PlaceHolder::__new(Ok(()))
}

// #[rpc_genie::rpc]
fn shutdown() -> PlaceHolder<Result<(), PlaceHolderError>> {
    PlaceHolder::__new(Ok(()))
}
