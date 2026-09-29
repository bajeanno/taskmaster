use shared_code::rpc::ClientHandle;

use crate::commands::placeholder::*;

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
    pub async fn send(&self, _client: &ClientHandle) -> Result<(), PlaceHolderError> {
        match self {
            Command::ListTasks => {
                list_tasks()
                    .call(_client)
                    .await?
                    .into_iter()
                    .for_each(|item| println!("\t{item}"));
            }
            Command::StartProgram(task) => {
                start(task.to_owned()).call(_client).await?.unwrap(); //TODO: check value at unwrap
            }
            Command::StopProgram(task) => {
                stop(task.to_owned()).call(_client).await?.unwrap(); //TODO: check value at unwrap
            }
            Command::RestartProgram(task) => {
                restart(task.to_owned()).call(_client).await?.unwrap(); //TODO: check value at unwrap
            }
            Command::ReloadConfigFile => {
                reload().call(_client).await?.unwrap(); //TODO: check value at unwrap
            }
            Command::StopDaemon => {
                shutdown().call(_client).await?.unwrap(); //TODO: check value at unwrap
            }
        }
        Ok(())
    }
}
