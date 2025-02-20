use serde::{Deserialize};
use serde_yaml::Value;
use std::{collections::HashMap, fmt::Display};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub programs: HashMap<String, Program>,
}

#[derive(Debug, Deserialize)]
pub struct Program {
    name: Option<String>,
    pid: Option<u32>,
    cmd: Option<String>,
    numprocs: Option<u32>,
    workingdir: Option<String>,
    autostart: Option<bool>,
    exitcodes: Option<Vec<u8>>, // check for valid codes (%256)
    startretries: Option<u32>,
    starttime: Option<u32>,
    stopsignal: Option<String>, // check for valid signal
    stoptime: Option<u32>,
    stdout: Option<String>,
    stderr: Option<String>,
    env: Option<HashMap<String, Value>>,
}

impl Config {
    pub fn update(&mut self) {
        for (key, value) in &mut self.programs {
            value.update(key.clone());
        }
    }
}

impl Program {
    fn update(&mut self, name: String) {
        if let None = self.name {
            self.name = Some(name);
        }
    }
}

impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:<15}{: ^10}{:10}",
            self.name.clone().unwrap_or_default(),
            self.pid.clone().unwrap_or_default(),
            self.cmd.clone().unwrap_or_default(),
        )
    }
}
