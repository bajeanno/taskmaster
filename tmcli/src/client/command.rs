use std::fmt;

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

pub struct Error;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "An error occurred")
    }
}

impl Command {
    pub async fn send(&self, _conn: Session) -> Result<(), Error> {
        match self {
            Command::ListTasks => {
                list_tasks(_conn).await.into_iter().for_each(|item| {
                    print!("\t{item}\n")
                });
            }
            Command::StartProgram(_) => {
                start(_conn).await?;
            }
            Command::StopProgram(_) => {
                stop(_conn).await?;
            }
            Command::RestartProgram(_) => {
                restart(_conn).await?;
            }
            Command::ReloadConfigFile => {
                reload(_conn).await?;
            }
            Command::StopDaemon => {
                shutdown(_conn).await?;
            }
        }
        Ok(())
    }
}

async fn list_tasks(_conn: Session) -> Vec<String> {
    vec!["nginx".to_string(), "transcendence".to_string()]
}

async fn start(_conn: Session) -> Result<(), Error> {
    Ok(())
}

async fn stop(_conn: Session) -> Result<(), Error> {
    Ok(())
}

async fn restart(_conn: Session) -> Result<(), Error> {
    Ok(())
}

async fn reload(_conn: Session) -> Result<(), Error> {
    Ok(())
}

async fn shutdown(_conn: Session) -> Result<(), Error> {
    Ok(())
}
