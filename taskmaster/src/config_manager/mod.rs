use std::sync::Arc;

use crate::config::Config;

use ConfigState::{Active, Uninitialized};

#[allow(dead_code)]
enum ConfigState {
    Active(Arc<Config>),
    Uninitialized,
    LoadError { error: String },
}

#[allow(dead_code)]
pub struct ConfigManager {
    state: ConfigState,
    last_reload_error: Option<String>,
}

#[allow(dead_code)]
impl ConfigManager {
    pub fn new() -> Self {
        Self {
            state: Uninitialized,
            last_reload_error: None,
        }
    }

    pub fn load_config(&mut self, file: Option<&str>) {
        match Config::parse(file.unwrap_or("taskmaster.yaml")) {
            Ok(config) => self.state = Active(Arc::new(config)),
            Err(err) => {
                eprintln!("Warning {err}"); //TODO: log error and/or broadcast to clients
                self.last_reload_error = Some(err.to_string());
            }
        };
    }
}
