use std::io::Write;
use std::sync::{Mutex, mpsc};
use std::time::Duration;

use super::*;

static PID_TEST_LOCK: Mutex<()> = Mutex::new(());

fn write_pid_file(content: &str) {
    let mut file = acquire_file_lock(PID_FILE).unwrap();
    file.set_len(0).unwrap();
    file.write_all(content.as_bytes()).unwrap();
    release_file_lock(file);
}

#[test]
fn test_read_pid_returns_some_for_valid_pid() {
    let _guard = PID_TEST_LOCK.lock().unwrap();
    write_pid_file("12345");
    let mut file = acquire_file_lock(PID_FILE).unwrap();
    assert_eq!(read_pid(&mut file).unwrap(), Some(12345));
    release_file_lock(file);
}

#[test]
fn test_read_pid_returns_none_for_empty_file() {
    let _guard = PID_TEST_LOCK.lock().unwrap();
    write_pid_file("");
    let mut file = acquire_file_lock(PID_FILE).unwrap();
    assert_eq!(read_pid(&mut file).unwrap(), None);
    release_file_lock(file);
}

#[test]
fn test_read_pid_returns_error_for_invalid_content() {
    let _guard = PID_TEST_LOCK.lock().unwrap();
    write_pid_file("taskmaster");
    let mut file = acquire_file_lock(PID_FILE).unwrap();
    assert!(matches!(
        read_pid(&mut file),
        Err(Error::Pid(PidError::Parse(_)))
    ));
    release_file_lock(file);
}

#[test]
fn test_read_pid_returns_error_when_read_fails() {
    let mut file = File::open(std::env::temp_dir()).unwrap();
    assert!(matches!(
        read_pid(&mut file),
        Err(Error::Pid(PidError::ReadFile(_)))
    ));
}

#[test]
fn test_claim_taskmaster_instance_writes_pid_when_free() {
    let _guard = PID_TEST_LOCK.lock().unwrap();
    write_pid_file("");
    let _claim = claim_taskmaster_instance().unwrap();
    let mut file = acquire_file_lock(PID_FILE).unwrap();
    assert_eq!(read_pid(&mut file).unwrap(), Some(std::process::id()));
    release_file_lock(file);
}

#[test]
fn test_claim_taskmaster_instance_fails_when_pid_present() {
    let _guard = PID_TEST_LOCK.lock().unwrap();
    write_pid_file("999999");
    let err = claim_taskmaster_instance().unwrap_err();
    assert!(matches!(err, Error::Pid(PidError::OtherInstanceRunning)));
}

#[test]
fn test_claim_taskmaster_instance_allows_only_one_instance() {
    let _guard = PID_TEST_LOCK.lock().unwrap();
    write_pid_file("");
    let first = std::thread::spawn(claim_taskmaster_instance);
    let second = std::thread::spawn(claim_taskmaster_instance);
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
        claim_taskmaster_instance().is_ok(),
        "releasing the claim must erase the pid file"
    );
}

#[test]
fn test_flock_blocks_second_lock_until_release() {
    let _guard = PID_TEST_LOCK.lock().unwrap();
    write_pid_file("");
    let first = acquire_file_lock(PID_FILE).unwrap();

    let (sender, receiver) = mpsc::channel();
    let second = std::thread::spawn(move || {
        let file = acquire_file_lock(PID_FILE).unwrap();
        release_file_lock(file);
        sender.send(()).unwrap();
    });

    assert!(
        receiver.recv_timeout(Duration::from_millis(150)).is_err(),
        "flock must block a second lock while the first is still held"
    );

    release_file_lock(first);

    receiver
        .recv_timeout(Duration::from_secs(2))
        .expect("flock must be granted once the first lock is released");
    second.join().unwrap();
}

#[test]
fn test_unclaim_taskmaster_instance_truncates_file() {
    let _guard = PID_TEST_LOCK.lock().unwrap();
    write_pid_file("12345");
    unclaim_taskmaster_instance();
    let mut file = acquire_file_lock(PID_FILE).unwrap();
    assert_eq!(read_pid(&mut file).unwrap(), None);
    release_file_lock(file);
}

#[test]
fn test_claim_taskmaster_instance_erases_file_on_drop() {
    let _guard = PID_TEST_LOCK.lock().unwrap();
    write_pid_file("");
    let claim = claim_taskmaster_instance().unwrap();
    let mut file = acquire_file_lock(PID_FILE).unwrap();
    assert!(read_pid(&mut file).unwrap().is_some());
    release_file_lock(file);

    drop(claim);

    let mut file = acquire_file_lock(PID_FILE).unwrap();
    assert_eq!(read_pid(&mut file).unwrap(), None);
    release_file_lock(file);
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
