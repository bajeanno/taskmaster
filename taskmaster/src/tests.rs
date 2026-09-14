use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use super::*;

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "taskmaster_pid_file_test_{}_{}",
            std::process::id(),
            name
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self, name: &str) -> String {
        self.0.join(name).to_str().unwrap().to_string()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn test_claim_pid_returns_some_for_valid_pid() {
    let tmp = TempDir::new("valid_pid");
    let path = tmp.path("pid");
    fs::write(&path, "12345").unwrap();
    let mut file = File::open(&path).unwrap();
    assert_eq!(claim_pid(&mut file).unwrap(), Some(12345));
}

#[test]
fn test_claim_pid_returns_none_for_empty_file() {
    let tmp = TempDir::new("empty_pid");
    let path = tmp.path("pid");
    fs::write(&path, "").unwrap();
    let mut file = File::open(&path).unwrap();
    assert_eq!(claim_pid(&mut file).unwrap(), None);
}

#[test]
fn test_claim_pid_returns_none_for_invalid_content() {
    let tmp = TempDir::new("invalid_pid");
    let path = tmp.path("pid");
    fs::write(&path, "taskmaster").unwrap();
    let mut file = File::open(&path).unwrap();
    assert_eq!(claim_pid(&mut file).unwrap(), None);
}

#[test]
fn test_claim_pid_returns_error_when_read_fails() {
    let tmp = TempDir::new("unreadable_pid");
    let mut file = File::open(&tmp.0).unwrap();
    assert!(matches!(
        claim_pid(&mut file),
        Err(Error::Pid(PidError::File(_)))
    ));
}

#[test]
fn test_check_already_running_in_writes_pid_when_free() {
    let tmp = TempDir::new("write_pid");
    let path = tmp.path("pid");
    check_already_running(&path).unwrap();
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        std::process::id().to_string()
    );
}

#[test]
fn test_check_already_running_in_fails_when_pid_present() {
    let tmp = TempDir::new("already_running");
    let path = tmp.path("pid");
    fs::write(&path, "999999").unwrap();
    let err = check_already_running(&path).unwrap_err();
    assert!(matches!(err, Error::Pid(PidError::OtherInstanceRunning)));
}

#[test]
fn test_check_already_running_in_allows_only_one_instance() {
    let tmp = TempDir::new("single_instance");
    let path = tmp.path("pid");
    let path_a = path.clone();
    let path_b = path.clone();
    let first = std::thread::spawn(move || check_already_running(&path_a));
    let second = std::thread::spawn(move || check_already_running(&path_b));
    let results = [first.join().unwrap(), second.join().unwrap()];
    let claimed = results.iter().filter(|result| result.is_ok()).count();
    let refused = results
        .iter()
        .filter(|result| matches!(result, Err(Error::Pid(PidError::OtherInstanceRunning))))
        .count();
    assert_eq!(claimed, 1, "exactly one instance must claim the pid file");
    assert_eq!(refused, 1, "the second instance must be refused");
}

#[test]
fn test_flock_blocks_second_lock_until_release() {
    let tmp = TempDir::new("lock_blocking");
    let path = tmp.path("pid");
    fs::write(&path, "").unwrap();

    let first = File::open(&path).unwrap();
    unsafe { flock(first.as_raw_fd(), LOCK_EX) };

    let (sender, receiver) = mpsc::channel();
    let second_path = path.clone();
    let second = std::thread::spawn(move || {
        let file = File::open(second_path).unwrap();
        unsafe { flock(file.as_raw_fd(), LOCK_EX) };
        sender.send(()).unwrap();
    });

    assert!(
        receiver.recv_timeout(Duration::from_millis(150)).is_err(),
        "flock must block a second lock while the first is still held"
    );

    unsafe { flock(first.as_raw_fd(), LOCK_UN) };

    receiver
        .recv_timeout(Duration::from_secs(2))
        .expect("flock must be granted once the first lock is released");
    second.join().unwrap();
}

#[test]
fn test_erase_pid_file_in_truncates_file() {
    let tmp = TempDir::new("erase");
    let path = tmp.path("pid");
    fs::write(&path, "12345").unwrap();
    erase_file(&path);
    assert_eq!(fs::read_to_string(&path).unwrap(), "");
}

#[test]
fn test_parse_args() {
    let mut port = Some("4444".to_string());
    assert_eq!(4444, parse_args(port).unwrap().port);
    port = Some("4443".to_string());
    assert_eq!(4443, parse_args(port).unwrap().port);
    port = Some("0".to_string());
    assert_eq!(0, parse_args(port).unwrap().port);
    port = Some("55".to_string());
    assert_eq!(55, parse_args(port).unwrap().port);

    assert_eq!(DEFAULT_PORT, parse_args(None).unwrap().port);

    port = Some("hey".to_string());
    let Err(Error::PortArgumentIsNotAnInteger { input, error: _ }) = parse_args(port) else {
        panic!("Function parse_args did not return an error")
    };
    assert_eq!(input, "hey");
}
