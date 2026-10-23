use std::{os::raw::c_int, process::ExitCode, thread::sleep, time::Duration, vec};

use libc::{
    signal::kill,
    sys::{
        types::Pid,
        wait::{waitpid, wexitstatus, wifexited},
    },
    unistd::fork,
};

#[test]
fn integration_tests() {
    eprintln!("running test 1");
    assert_eq!(cannot_start_after_crash(), ExitCode::SUCCESS);
    eprintln!("finished test 1");

    eprintln!("running test 2");
    assert_eq!(force_start(), ExitCode::SUCCESS);
    eprintln!("finished test 2");

    eprintln!("running test 3");
    assert_eq!(check_force_start_kills_running_daemon(), ExitCode::SUCCESS);
    eprintln!("finished test 3");

    eprintln!("running test 4");
    assert_eq!(check_sigint_kills_running_daemon(), ExitCode::SUCCESS);
    eprintln!("finished test 4");
}

fn cannot_start_after_crash() -> ExitCode {
    let pid = unsafe { start_taskmaster_in_fork(vec!["-f".to_string(), "-d".to_string()]) };

    sleep(Duration::from_millis(400));
    kill_taskmaster_daemon(pid);

    let mut status: c_int = 0;
    let option: c_int = 0;
    unsafe { waitpid(pid, &mut status, option) };
    assert!(
        !unsafe { wifexited(status) },
        "process exited before being killed"
    );

    assert_eq!(
        unsafe { taskmaster::entrypoint(vec!["-y".to_string()]) },
        ExitCode::FAILURE
    );
    ExitCode::SUCCESS
}

fn force_start() -> ExitCode {
    let pid = unsafe {
        start_taskmaster_in_fork(vec!["--force".to_string(), "--daemon-only".to_string()])
    };

    sleep(Duration::from_millis(400));
    unsafe { shutdown_taskmaster_daemon_cleanly(pid) };

    let mut status: c_int = 0;
    let option: c_int = 0;

    unsafe { waitpid(pid, &mut status, option) };
    assert!(
        unsafe { wifexited(status) },
        "process exited before being killed"
    );
    assert_eq!(unsafe { wexitstatus(status) }, 0);

    ExitCode::SUCCESS
}

fn check_force_start_kills_running_daemon() -> ExitCode {
    let pid = unsafe { start_taskmaster_in_fork(vec!["-y".to_string(), "-d".to_string()]) };

    sleep(Duration::from_millis(400));
    let pid_force = unsafe {
        start_taskmaster_in_fork(vec!["--force".to_string(), "--daemon-only".to_string()])
    };

    let mut status: c_int = 0;
    let option: c_int = 0;

    unsafe { waitpid(pid, &mut status, option) };

    if unsafe { wifexited(status) } {
        return ExitCode::FAILURE;
        // panic!("process exited before being killed");
    }

    unsafe { shutdown_taskmaster_daemon_cleanly(pid_force) };

    let mut status: c_int = 0;
    let option: c_int = 0;
    unsafe { waitpid(pid_force, &mut status, option) };
    ExitCode::SUCCESS
}

fn check_sigint_kills_running_daemon() -> ExitCode {
    let pid = unsafe { start_taskmaster_in_fork(vec!["-y".to_string(), "-d".to_string()]) };
    sleep(Duration::from_millis(400));

    unsafe {
        kill(pid, signal::Signal::SIGINT as c_int);
    }

    let mut status: c_int = 0;
    let option: c_int = 0;

    unsafe { waitpid(pid, &mut status, option) };
    ExitCode::SUCCESS
}

unsafe fn start_taskmaster_in_fork(args: Vec<String>) -> Pid {
    let pid = unsafe { fork() };
    assert!(pid >= 0);

    if pid == 0 {
        match unsafe { taskmaster::entrypoint(args) } {
            ExitCode::SUCCESS => std::process::exit(0),
            ExitCode::FAILURE => std::process::exit(1),
            _ => std::process::exit(2),
        }
    }

    pid
}

unsafe fn shutdown_taskmaster_daemon_cleanly(taskmaster_pid: Pid) {
    unsafe { kill(taskmaster_pid, signal::Signal::SIGINT as c_int) };
}

fn kill_taskmaster_daemon(taskmaster_pid: Pid) {
    unsafe { kill(taskmaster_pid, signal::Signal::SIGKILL as c_int) };
}
