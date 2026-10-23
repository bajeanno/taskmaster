use std::{os::raw::c_int, process::ExitCode, thread::sleep, time::Duration, vec};

use libc::{
    signal::kill,
    sys::{
        types::Pid,
        wait::{waitpid, wifexited},
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
    // eprintln!("running test 3");
    // assert_eq!(check_force_start_kills_running_daemon(), ExitCode::SUCCESS);
    // eprintln!("finished test 3");
    // eprintln!("running test 4");
    // assert_eq!(check_sigint_kills_running_daemon(), ExitCode::SUCCESS);
    // eprintln!("finished test 4");
}

fn cannot_start_after_crash() -> ExitCode {
    let pid = unsafe { start_taskmaster(vec!["-f".to_string()]) };

    sleep(Duration::from_millis(400));
    unsafe {
        // SIGKILL as it is intentionnal crash
        kill(pid as c_int, signal::Signal::SIGKILL as c_int)
    };

    let status: c_int = 0;
    let option: c_int = 0;
    unsafe { waitpid(pid, &status, option) };
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
    let pid = unsafe { start_taskmaster(vec!["--force".to_string()]) };

    sleep(Duration::from_millis(400)); // let time to start
    unsafe {
        kill(pid as c_int, signal::Signal::SIGINT as c_int);
    }
    sleep(Duration::from_millis(400)); // let time to unwind

    let status: c_int = 0;
    let option: c_int = 0;

    unsafe { waitpid(pid, &status, option) };
    if unsafe { wifexited(status) } {
        panic!("process exited before being killed");
    }
    ExitCode::SUCCESS
}

fn check_force_start_kills_running_daemon() -> ExitCode {
    let pid = unsafe { start_taskmaster(vec!["-y".to_string()]) };

    sleep(Duration::from_millis(400));
    let pid_force = unsafe { start_taskmaster(vec!["--force".to_string()]) };

    let status: c_int = 0;
    let option: c_int = 0;

    unsafe { waitpid(pid, &status, option) };
    if unsafe { wifexited(status) } {
        return ExitCode::FAILURE;
        // panic!("process exited before being killed");
    }

    unsafe {
        kill(pid_force as c_int, signal::Signal::SIGINT as c_int);
    }

    let status: c_int = 0;
    let option: c_int = 0;
    unsafe { waitpid(pid_force, &status, option) };
    ExitCode::SUCCESS
}

fn check_sigint_kills_running_daemon() -> ExitCode {
    let pid = unsafe { start_taskmaster(vec!["-y".to_string()]) };
    sleep(Duration::from_millis(400));

    unsafe {
        kill(pid as c_int, signal::Signal::SIGINT as c_int);
    }

    let status: c_int = 0;
    let option: c_int = 0;

    unsafe { waitpid(pid, &status, option) };
    ExitCode::SUCCESS
}

unsafe fn start_taskmaster(args: Vec<String>) -> Pid {
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
