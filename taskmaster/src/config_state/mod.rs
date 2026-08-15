use crate::config::Config;
use std::sync::Arc;

#[allow(dead_code)]
#[derive(Default)]
pub enum ConfigState {
    Active(Arc<Config>),
    #[default]
    Uninitialized,
    LoadError {
        error: String,
    },
}

impl ConfigState {
    #[cfg(test)]
    pub fn from_content(content: String) -> Self {
        use std::io::Cursor;

        let config = Config::from_reader(Cursor::new(content)).expect("Parse error");
        Self::Active(Arc::new(config))
    }

    pub fn from_config(file: Option<&str>) -> Self {
        let mut config = Self::default();
        config.load_config(file);
        config
    }

    pub fn load_config(&mut self, file: Option<&str>) {
        match Config::parse(file.unwrap_or("taskmaster.yaml")) {
            Ok(config) => *self = Self::Active(Arc::new(config)),
            Err(err) => {
                eprintln!("Warning {err}"); //TODO: log error and/or broadcast to clients
                *self = Self::LoadError {
                    error: err.to_string(),
                };
            }
        };
    }

    pub fn take(&mut self) -> Self {
        let mut tmp = Self::default();
        std::mem::swap(&mut tmp, self);
        tmp
    }
}
