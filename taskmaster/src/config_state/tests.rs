use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use super::{ConfFile, ConfigFileError, ConfigState, DEFAULT_TASKS_FILE};

const VALID_YAML: &str = r#"programs:
  testprog:
    cmd: "sleep 30"
    numprocs: 2"#;

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "taskmaster_config_state_test_{}_{}",
            std::process::id(),
            name
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn write(&self, name: &str, content: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, content).unwrap();
        path
    }

    fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn expect_active(state: &ConfigState) -> Arc<crate::config::Config> {
    match state {
        ConfigState::Active(config) => Arc::clone(config),
        _ => panic!("expected Active config state"),
    }
}

#[test]
fn test_conf_file_toml_round_trip() {
    let conf = ConfFile {
        config_file_path: "/tmp/tasks.yaml".to_string(),
    };
    let serialized = toml::to_string(&conf).unwrap();
    let deserialized: ConfFile = toml::from_str(&serialized).unwrap();
    assert_eq!(deserialized.config_file_path, "/tmp/tasks.yaml");
}

#[test]
fn test_register_new_config_file_writes_requested_path() {
    let tmp = TempDir::new("register_writes");
    let conf_path = tmp.join("conf.toml");
    let state = ConfigState::default();
    let registered = state
        .register_new_config_file(conf_path.to_str().unwrap(), Some("/tmp/tasks.yaml"))
        .unwrap();
    assert_eq!(registered, "/tmp/tasks.yaml");
    let content = fs::read_to_string(&conf_path).unwrap();
    let parsed: ConfFile = toml::from_str(&content).unwrap();
    assert_eq!(parsed.config_file_path, "/tmp/tasks.yaml");
}

#[test]
fn test_register_new_config_file_defaults_to_default_tasks_file() {
    let tmp = TempDir::new("register_default");
    let conf_path = tmp.join("conf.toml");
    let state = ConfigState::default();
    let registered = state
        .register_new_config_file(conf_path.to_str().unwrap(), None)
        .unwrap();
    assert_eq!(registered, DEFAULT_TASKS_FILE);
}

#[test]
fn test_register_new_config_file_fails_when_conf_already_exists() {
    let tmp = TempDir::new("register_exists");
    let conf_path = tmp.write("conf.toml", "config_file_path = \"/tmp/old.yaml\"\n");
    let state = ConfigState::default();
    let result = state.register_new_config_file(conf_path.to_str().unwrap(), Some("/tmp/new.yaml"));
    assert!(matches!(result, Err(ConfigFileError::Open(_))));
}

#[test]
fn test_fetch_tasks_file_path_reads_stored_path() {
    let tmp = TempDir::new("fetch_ok");
    let conf_path = tmp.write("conf.toml", "config_file_path = \"/tmp/tasks.yaml\"\n");
    let state = ConfigState::default();
    let fetched = state
        .fetch_tasks_file_path(conf_path.to_str().unwrap())
        .unwrap();
    assert_eq!(fetched, "/tmp/tasks.yaml");
}

#[test]
fn test_fetch_tasks_file_path_fails_on_invalid_toml() {
    let tmp = TempDir::new("fetch_invalid_toml");
    let conf_path = tmp.write("conf.toml", "not valid toml [");
    let state = ConfigState::default();
    let result = state.fetch_tasks_file_path(conf_path.to_str().unwrap());
    assert!(matches!(result, Err(ConfigFileError::Parse(_))));
}

#[test]
fn test_fetch_tasks_file_path_fails_on_missing_field() {
    let tmp = TempDir::new("fetch_missing_field");
    let conf_path = tmp.write("conf.toml", "");
    let state = ConfigState::default();
    let result = state.fetch_tasks_file_path(conf_path.to_str().unwrap());
    assert!(matches!(result, Err(ConfigFileError::Parse(_))));
}

#[test]
fn test_load_config_with_explicit_file_activates_config() {
    let tmp = TempDir::new("load_explicit");
    let tasks_path = tmp.write("tasks.yaml", VALID_YAML);
    let conf_path = tmp.join("conf.toml");
    let mut state = ConfigState::default();
    state
        .load_config_with(
            conf_path.to_str().unwrap(),
            Some(tasks_path.to_str().unwrap()),
        )
        .unwrap();
    let config = expect_active(&state);
    assert!(config.programs.contains_key("testprog"));
}

#[test]
fn test_load_config_registers_conf_file_for_later_reload() {
    let tmp = TempDir::new("load_registers");
    let tasks_path = tmp.write("tasks.yaml", VALID_YAML);
    let conf_path = tmp.join("conf.toml");
    let mut state = ConfigState::default();
    state
        .load_config_with(
            conf_path.to_str().unwrap(),
            Some(tasks_path.to_str().unwrap()),
        )
        .unwrap();
    let mut reloaded = ConfigState::default();
    reloaded
        .load_config_with(conf_path.to_str().unwrap(), None)
        .unwrap();
    let original = expect_active(&state);
    let reloaded_config = expect_active(&reloaded);
    assert_eq!(*reloaded_config, *original);
}

#[test]
fn test_load_config_sets_load_error_when_yaml_file_missing() {
    let tmp = TempDir::new("load_missing_yaml");
    let conf_path = tmp.join("conf.toml");
    let missing_path = tmp.join("missing.yaml");
    let mut state = ConfigState::default();
    state
        .load_config_with(
            conf_path.to_str().unwrap(),
            Some(missing_path.to_str().unwrap()),
        )
        .unwrap();
    match state {
        ConfigState::LoadError { error } => {
            assert!(error.contains("missing.yaml"), "unexpected error: {error}")
        }
        _ => panic!("expected LoadError"),
    }
}

#[test]
fn test_load_config_sets_load_error_when_yaml_is_invalid() {
    let tmp = TempDir::new("load_invalid_yaml");
    let tasks_path = tmp.write("tasks.yaml", "not: [valid");
    let conf_path = tmp.join("conf.toml");
    let mut state = ConfigState::default();
    state
        .load_config_with(
            conf_path.to_str().unwrap(),
            Some(tasks_path.to_str().unwrap()),
        )
        .unwrap();
    match state {
        ConfigState::LoadError { .. } => {}
        _ => panic!("expected LoadError"),
    }
}

#[test]
fn test_load_config_fails_when_registration_fails() {
    let tmp = TempDir::new("load_unwritable_conf");
    let tasks_path = tmp.write("tasks.yaml", VALID_YAML);
    let conf_path = tmp.join("nonexistent_dir/conf.toml");
    let mut state = ConfigState::default();
    let result = state.load_config_with(
        conf_path.to_str().unwrap(),
        Some(tasks_path.to_str().unwrap()),
    );
    assert!(matches!(result, Err(ConfigFileError::Open(_))));
    assert!(matches!(state, ConfigState::Uninitialized));
}

#[test]
fn test_load_config_fails_when_fetching_without_valid_conf_file() {
    let tmp = TempDir::new("load_no_conf");
    let conf_path = tmp.join("nonexistent_dir/conf.toml");
    let mut state = ConfigState::default();
    let result = state.load_config_with(conf_path.to_str().unwrap(), None);
    assert!(matches!(result, Err(ConfigFileError::Open(_))));
}

#[test]
fn test_take_returns_previous_state_and_resets_to_uninitialized() {
    let mut state = ConfigState::from_content(VALID_YAML.to_string());
    let taken = state.take();
    assert!(matches!(taken, ConfigState::Active(_)));
    assert!(matches!(state, ConfigState::Uninitialized));
}

#[test]
fn test_take_preserves_underlying_config() {
    let mut state = ConfigState::from_content(VALID_YAML.to_string());
    let original = expect_active(&state);
    let taken = state.take();
    match taken {
        ConfigState::Active(config) => assert!(Arc::ptr_eq(&config, &original)),
        _ => panic!("expected Active"),
    }
}

#[test]
fn test_error_display_messages() {
    let open_err = ConfigFileError::Open(io::Error::other("boom"));
    assert_eq!(
        open_err.to_string(),
        "Failed to open taskmaster configuration file: boom"
    );
    let read_err = ConfigFileError::Read(io::Error::other("boom"));
    assert_eq!(
        read_err.to_string(),
        "Failed to read taskmaster configuration file: boom"
    );
    let write_err = ConfigFileError::Write(io::Error::other("boom"));
    assert_eq!(
        write_err.to_string(),
        "Failed to write taskmaster configuration file: boom"
    );
    let parse_err = ConfigFileError::from(toml::from_str::<ConfFile>("[").unwrap_err());
    assert!(
        parse_err
            .to_string()
            .starts_with("Failed to parse taskmaster configuration file:")
    );
}
