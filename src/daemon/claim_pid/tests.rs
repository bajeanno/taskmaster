use std::{
    assert_matches,
    sync::Mutex,
};

use crate::daemon::{
    claim_pid::{Claim, PidFile},
    error::ClaimError,
    test_utils::TestDir,
};
use crate::start_mode::StartMode;

static PID_TEST_LOCK: Mutex<()> = Mutex::new(());

fn set_pid_file_content(content: &str) -> (TestDir, String) {
    let dir = TestDir::new("pid").sub_dir("set_content");
    let path = dir.join("taskmaster.pid");
    let mut pid_file = PidFile::open(&path).unwrap();
    pid_file.set_content_for_tests(content);
    (dir, path)
}

#[test]
fn test_read_pid_returns_some_for_valid_pid() {
    let _guard = PID_TEST_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let (_dir, path) = set_pid_file_content("12345");
    let mut pid_file = PidFile::open(&path).unwrap();
    assert_eq!(pid_file.with_lock(|file| unsafe {file.read_pid()}).unwrap(), Some(12345));
}

#[test]
fn test_read_pid_returns_none_for_empty_file() {
    let _guard = PID_TEST_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let (_dir, path) = set_pid_file_content("");
    let mut pid_file = PidFile::open(&path).unwrap();
    assert_eq!(pid_file.with_lock(|file| unsafe { file.read_pid() }).unwrap(), None);
}

#[test]
fn test_read_pid_returns_error_for_invalid_content() {
    let _guard = PID_TEST_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let (_dir, path) = set_pid_file_content("taskmaster");
    let mut pid_file = PidFile::open(&path).unwrap();
    assert_matches!(pid_file.with_lock(|file| unsafe { file.read_pid() }), Err(ClaimError::Parse(_)));
}

#[test]
fn test_claim_writes_pid_when_free() {
    let _guard = PID_TEST_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let (_dir, path) = set_pid_file_content("");
    let mut claim = Claim::new_at(&path, StartMode::Classic).unwrap();
    assert_eq!(
        claim.file.with_lock(|file| unsafe { file.read_pid() }).unwrap(),
        Some(std::process::id())
    );
}

#[test]
fn test_claim_fails_when_pid_present() {
    let _guard = PID_TEST_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let (_dir, path) = set_pid_file_content("999999");
    let err = Claim::new_at(&path, StartMode::Classic).unwrap_err();
    assert_matches!(err, ClaimError::OtherInstanceRunning);
}

#[test]
fn test_claim_allows_only_one_instance() {
    let _guard = PID_TEST_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let (_dir, path) = set_pid_file_content("");

    let first_path = path.clone();
    let second_path = path.clone();
    let first = std::thread::spawn(move || Claim::new_at(&first_path, StartMode::Classic));
    let second = std::thread::spawn(move || Claim::new_at(&second_path, StartMode::Classic));
    let results = [first.join().unwrap(), second.join().unwrap()];

    let mut claimed = vec![];
    let mut refused = 0;
    for result in results {
        match result {
            Ok(claim) => claimed.push(claim),
            Err(ClaimError::OtherInstanceRunning) => refused += 1,
            Err(other) => panic!("unexpected error: {other:?}"),
        }
    }

    assert_eq!(
        claimed.len(),
        1,
        "exactly one instance must claim the pid file"
    );
    assert_eq!(refused, 1, "the second instance must be refused");

    drop(claimed);
    assert!(
        Claim::new_at(&path, StartMode::Classic).is_ok(),
        "releasing the claim must erase the pid file"
    );
}

#[test]
fn test_pid_file_truncate_clears_content() {
    let _guard = PID_TEST_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let (_dir, path) = set_pid_file_content("12345");
    let mut pid_file = PidFile::open(&path).unwrap();
    let _ = pid_file.with_lock(|file| {
        unsafe { file.truncate() };
        Ok::<(), ClaimError>(())
    });
    drop(pid_file);

    let mut pid_file = PidFile::open(&path).unwrap();
    assert_eq!(pid_file.with_lock(|file| unsafe { file.read_pid() }).unwrap(), None);
}

#[test]
fn test_claim_erases_file_on_drop() {
    let _guard = PID_TEST_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let (_dir, path) = set_pid_file_content("");

    let claim = Claim::new_at(&path, StartMode::Classic).unwrap();
    let mut pid_file = PidFile::open(&path).unwrap();
    assert!(pid_file.with_lock(|file| unsafe { file.read_pid() }).unwrap().is_some());
    drop(pid_file);

    drop(claim);

    let mut pid_file = PidFile::open(&path).unwrap();
    assert_eq!(pid_file.with_lock(|file| unsafe { file.read_pid() }).unwrap(), None);
}
