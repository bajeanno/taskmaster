use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time went backwards")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("taskmaster-{name}-{}-{unique}", std::process::id()));
        fs::create_dir_all(&path).expect("Failed to create temp directory");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run_taskmaster(args: &[&str], cwd: &Path) -> Output {
    Command::new(
        std::env::var("CARGO_BIN_EXE_taskmaster")
            .expect("CARGO_BIN_EXE_taskmaster is not set for integration tests"),
    )
    .args(args)
    .current_dir(cwd)
    .output()
    .expect("Failed to run taskmaster binary")
}

#[test]
fn prints_port_parse_error_message() {
    let temp_dir = TempDir::new("port-parse-error");
    let output = run_taskmaster(&["not-a-number"], temp_dir.path());
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stderr.contains("Failed to parse port number from input: 'not-a-number':"),
        "stderr was: {stderr}"
    );
}

#[test]
fn prints_warning_for_missing_config_file() {
    let temp_dir = TempDir::new("missing-config");
    let output = run_taskmaster(&[], temp_dir.path());
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stderr.contains("Warning Error opening taskmaster config file: taskmaster.yaml:"),
        "stderr was: {stderr}"
    );
    assert!(
        stderr.contains("Consider making a reload request after fixing the issue"),
        "stderr was: {stderr}"
    );
}

#[test]
fn prints_warning_for_invalid_yaml_config() {
    let temp_dir = TempDir::new("invalid-yaml");
    fs::write(temp_dir.path().join("taskmaster.yaml"), "programs:\n  bad: [\n")
        .expect("Failed to write invalid config file");

    let output = run_taskmaster(&[], temp_dir.path());
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stderr.contains("Warning Error parsing taskmaster config file: taskmaster.yaml:"),
        "stderr was: {stderr}"
    );
    assert!(
        stderr.contains("Consider making a reload request after fixing the issue"),
        "stderr was: {stderr}"
    );
}

#[test]
fn prints_warning_for_empty_command_error() {
    let temp_dir = TempDir::new("empty-command");
    fs::write(
        temp_dir.path().join("taskmaster.yaml"),
        "programs:\n  bad:\n    cmd: \"\"\n",
    )
    .expect("Failed to write config file");

    let output = run_taskmaster(&[], temp_dir.path());
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stderr.contains("Warning Error parsing taskmaster config file: taskmaster.yaml:"),
        "stderr was: {stderr}"
    );
    assert!(
        stderr.contains("Command parsing error: Empty command"),
        "stderr was: {stderr}"
    );
}

#[test]
fn prints_warning_for_split_command_error() {
    let temp_dir = TempDir::new("split-command");
    fs::write(
        temp_dir.path().join("taskmaster.yaml"),
        "programs:\n  bad:\n    cmd: 'echo \"unterminated'\n",
    )
    .expect("Failed to write config file");

    let output = run_taskmaster(&[], temp_dir.path());
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stderr.contains("Warning Error parsing taskmaster config file: taskmaster.yaml:"),
        "stderr was: {stderr}"
    );
    assert!(
        stderr.contains("Command parsing error:"),
        "stderr was: {stderr}"
    );
    assert!(stderr.contains("quote"), "stderr was: {stderr}");
}
