use crate::TestDir;
use std::sync::Arc;
use std::{assert_matches, io};

use super::{ConfigState, DEFAULT_TASKS_FILE, InitFile, InitFileError, ReloadArgs};

const VALID_YAML: &str = r#"programs:
  testprog:
    cmd: "sleep 30"
    num-procs: 2"#;

fn expect_active(state: &ConfigState) -> Arc<crate::config::Config> {
    match state {
        ConfigState::Active {
            config,
            config_file_path: _,
        } => Arc::clone(config),
        _ => panic!("expected Active config state"),
    }
}

#[test]
fn test_init_file_new_uses_default_paths() {
    let init_file = InitFile::new();
    assert_eq!(init_file.default_config_file_path, DEFAULT_TASKS_FILE);
}

#[test]
fn test_init_file_ron_round_trip() {
    let init_file = InitFile {
        default_config_file_path: "/tmp/default.yaml".to_string(),
    };
    let serialized = ron::to_string(&init_file).unwrap();
    let deserialized: InitFile = ron::from_str(&serialized).unwrap();
    assert_eq!(deserialized.default_config_file_path, "/tmp/default.yaml");
}

#[test]
fn test_from_content_activates_config() {
    let state = ConfigState::from_content(VALID_YAML.to_string());
    let config = expect_active(&state);
    assert!(config.programs.contains_key("testprog"));
}

#[test]
fn test_take_returns_previous_state_and_resets_to_uninitialized() {
    let mut state = ConfigState::from_content(VALID_YAML.to_string());
    let taken = state.take();
    assert_matches!(
        taken,
        ConfigState::Active {
            config: _,
            config_file_path: _
        }
    );
    assert_matches!(state, ConfigState::Uninitialized);
}

#[test]
fn test_take_preserves_underlying_config() {
    let mut state = ConfigState::from_content(VALID_YAML.to_string());
    let original = expect_active(&state);
    let taken = state.take();
    match taken {
        ConfigState::Active {
            config,
            config_file_path: _,
        } => assert!(Arc::ptr_eq(&config, &original)),
        _ => panic!("expected Active"),
    }
}

#[test]
fn test_error_display_messages() {
    let open_err = InitFileError::Open(io::Error::other("boom"));
    assert_eq!(
        open_err.to_string(),
        "Failed to open taskmaster configuration file: boom"
    );
    let read_err = InitFileError::Read(io::Error::other("boom"));
    assert_eq!(
        read_err.to_string(),
        "Failed to read taskmaster configuration file: boom"
    );
    let write_err = InitFileError::Write(io::Error::other("boom"));
    assert_eq!(
        write_err.to_string(),
        "Failed to write taskmaster configuration file: boom"
    );
    let parse_err = InitFileError::from(ron::from_str::<InitFile>("[").unwrap_err());
    assert!(
        parse_err
            .to_string()
            .starts_with("Failed to parse taskmaster configuration file:")
    );
}

#[test]
fn test_load_config_temp_config_returns_active_with_file_path() {
    let tmp = TestDir::new("temp_config");
    let tasks_path = tmp.write("tasks.yaml", VALID_YAML);
    let state = ConfigState::default()
        .load_config(ReloadArgs::TempConfig(tasks_path.clone()))
        .unwrap();
    match state {
        ConfigState::Active {
            config,
            config_file_path,
        } => {
            assert!(config.programs.contains_key("testprog"));
            assert_eq!(config_file_path, tasks_path);
        }
        _ => panic!("expected Active config state"),
    }
}

#[test]
fn test_load_config_use_current_reloads_current_file() {
    let tmp = TestDir::new("use_current");
    let tasks_path = tmp.write("tasks.yaml", VALID_YAML);
    let active = ConfigState::default()
        .load_config(ReloadArgs::TempConfig(tasks_path.clone()))
        .unwrap();
    let reloaded = active.load_config(ReloadArgs::UseCurrent).unwrap();
    match reloaded {
        ConfigState::Active {
            config,
            config_file_path,
        } => {
            assert!(config.programs.contains_key("testprog"));
            assert_eq!(config_file_path, tasks_path);
        }
        _ => panic!("expected Active config state"),
    }
}

#[test]
fn test_load_config_missing_file_creates_template() {
    let tmp = TestDir::new("missing_file_creates_template");
    let missing_path = tmp.join("not_created.yaml");
    let state = ConfigState::default()
        .load_config(ReloadArgs::TempConfig(missing_path.clone()))
        .unwrap();
    match state {
        ConfigState::Active {
            config,
            config_file_path,
        } => {
            assert!(config.programs.contains_key("template_task"));
            assert_eq!(config_file_path, missing_path);
        }
        _ => panic!("expected Active config state"),
    }
}

#[test]
fn test_load_config_invalid_yaml_returns_load_error() {
    let tmp = TestDir::new("invalid_yaml");
    let tasks_path = tmp.write("tasks.yaml", "not: [valid");
    let state = ConfigState::default()
        .load_config(ReloadArgs::TempConfig(tasks_path))
        .unwrap();
    assert_matches!(state, ConfigState::LoadError { .. });
}
