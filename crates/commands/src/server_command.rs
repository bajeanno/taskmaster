use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServerCommand {
    ListTasks,
<<<<<<< HEAD
    StartProgram(String),
    StopProgram(String),
    RestartProgram(String),
    ReloadConfigFile,
    StopDaemon,
=======
>>>>>>> main
}
