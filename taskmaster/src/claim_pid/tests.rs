use std::{sync::{Mutex, mpsc}, time::Duration};

use crate::{claim_pid::{Claim, PidFile}, error::{Error, PidError}};

static PID_TEST_LOCK: Mutex<()> = Mutex::new(());

fn set_pid_file_content(content: &str) {
    let mut pid_file = PidFile::open().unwrap();
    pid_file.set_content_for_tests(content);
}

#[test]
fn test_read_pid_returns_some_for_valid_pid() {
    let _guard = PID_TEST_LOCK.lock().unwrap();
    set_pid_file_content("12345");
    let mut pid_file = PidFile::open().unwrap();
    assert_eq!(pid_file.read_pid().unwrap(), Some(12345));
}

#[test]
fn test_read_pid_returns_none_for_empty_file() {
    let _guard = PID_TEST_LOCK.lock().unwrap();
    set_pid_file_content("");
    let mut pid_file = PidFile::open().unwrap();
    assert_eq!(pid_file.read_pid().unwrap(), None);
}

#[test]
fn test_read_pid_returns_error_for_invalid_content() {
    let _guard = PID_TEST_LOCK.lock().unwrap();
    set_pid_file_content("taskmaster");
    let mut pid_file = PidFile::open().unwrap();
    assert!(matches!(
        pid_file.read_pid(),
        Err(Error::Pid(PidError::Parse(_)))
    ));
}

#[test]
fn test_claim_writes_pid_when_free() {
    let _guard = PID_TEST_LOCK.lock().unwrap();
    set_pid_file_content("");
    let _claim = Claim::new().unwrap();
    let mut pid_file = PidFile::open().unwrap();
    assert_eq!(pid_file.read_pid().unwrap(), Some(std::process::id()));
}

#[test]
fn test_claim_fails_when_pid_present() {
    let _guard = PID_TEST_LOCK.lock().unwrap();
    set_pid_file_content("999999");
    let err = Claim::new().unwrap_err();
    assert!(matches!(err, Error::Pid(PidError::OtherInstanceRunning)));
}

#[test]
fn test_claim_allows_only_one_instance() {
    let _guard = PID_TEST_LOCK.lock().unwrap();
    set_pid_file_content("");

    let first = std::thread::spawn(Claim::new);
    let second = std::thread::spawn(Claim::new);
    let results = [first.join().unwrap(), second.join().unwrap()];

    let mut claimed = vec![];
    let mut refused = 0;
    for result in results {
        match result {
            Ok(claim) => claimed.push(claim),
            Err(Error::Pid(PidError::OtherInstanceRunning)) => refused += 1,
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
        Claim::new().is_ok(),
        "releasing the claim must erase the pid file"
    );
}

#[test]
fn test_flock_blocks_second_lock_until_release() {
    let _guard = PID_TEST_LOCK.lock().unwrap();
    let first = PidFile::open().unwrap();

    let (sender, receiver) = mpsc::channel();
    let second = std::thread::spawn(move || {
        let _pid_file = PidFile::open().unwrap();
        sender.send(()).unwrap();
    });

    assert!(
        receiver.recv_timeout(Duration::from_millis(150)).is_err(),
        "flock must block a second lock while the first is still held"
    );

    drop(first);

    receiver
        .recv_timeout(Duration::from_secs(2))
        .expect("flock must be granted once the first lock is released");
    second.join().unwrap();
}

#[test]
fn test_pid_file_truncate_clears_content() {
    let _guard = PID_TEST_LOCK.lock().unwrap();
    let mut pid_file = PidFile::open().unwrap();
    pid_file.set_content_for_tests("12345");
    pid_file.truncate();
    drop(pid_file);

    let mut pid_file = PidFile::open().unwrap();
    assert_eq!(pid_file.read_pid().unwrap(), None);
}

#[test]
fn test_claim_erases_file_on_drop() {
    let _guard = PID_TEST_LOCK.lock().unwrap();
    set_pid_file_content("");

    let claim = Claim::new().unwrap();
    let mut pid_file = PidFile::open().unwrap();
    assert!(pid_file.read_pid().unwrap().is_some());
    drop(pid_file);

    drop(claim);

    let mut pid_file = PidFile::open().unwrap();
    assert_eq!(pid_file.read_pid().unwrap(), None);
}
