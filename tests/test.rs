use std::{os::raw::c_int, process::ExitCode, thread::sleep, time::Duration};

use libc::{
    signal::kill,
    sys::wait::{waitpid, wifexited},
    unistd::fork,
};

#[test]
fn integration_tests() {
    assert_eq!(cannot_start_after_crash(), ExitCode::SUCCESS);
    assert_eq!(force_start(), ExitCode::SUCCESS);
    assert_eq!(check_force_start_kills_running_daemon(), ExitCode::SUCCESS);
    assert_eq!(check_sigint_kills_running_daemon(), ExitCode::SUCCESS);
}

fn cannot_start_after_crash() -> ExitCode {
    // TODO: create a pipe to bypass daemon start prompt
    let pid = unsafe { fork() };
    if pid == 0 {
        println!("starting");
        return unsafe { taskmaster::main(vec!["-y".to_string()]) };
    }

    sleep(Duration::from_millis(500));
    unsafe {
        kill(pid as c_int, signal::Signal::SIGKILL as c_int);
    }

    let status: c_int = 0;
    let option: c_int = 0;
    unsafe { waitpid(pid, &status, option) };
    if unsafe { wifexited(status) } {
        panic!("process exited before being killed");
    }
    println!("starting");
    assert_eq!(
        unsafe { taskmaster::main(Vec::with_capacity(0)) },
        ExitCode::FAILURE
    );
    ExitCode::SUCCESS
}

fn force_start() -> ExitCode {
    let pid = unsafe { fork() };
    if pid == 0 {
        unsafe { taskmaster::main(vec!["--force".to_string()]) };
    }
    sleep(Duration::from_millis(400));
    unsafe {
        kill(pid as c_int, signal::Signal::SIGKILL as c_int);
    }

    let status: c_int = 0;
    let option: c_int = 0;

    unsafe { waitpid(pid, &status, option) };
    if unsafe { wifexited(status) } {
        panic!("process exited before being killed");
    }
    ExitCode::SUCCESS
}

fn check_force_start_kills_running_daemon() -> ExitCode {
    let pid = unsafe { fork() };
    if pid == 0 {
        unsafe { taskmaster::main(Vec::with_capacity(0)) };
    }
    sleep(Duration::from_millis(400));
    let pid_force = unsafe { fork() };
    if pid_force == 0 {
        unsafe { taskmaster::main(vec!["--force".to_string()]) };
    }

    let status: c_int = 0;
    let option: c_int = 0;

    unsafe { waitpid(pid, &status, option) };
    if unsafe { wifexited(status) } {
        panic!("process exited before being killed");
    }
    unsafe {
        kill(pid_force as c_int, signal::Signal::SIGKILL as c_int);
    }

    let status: c_int = 0;
    let option: c_int = 0;

    unsafe { waitpid(pid_force, &status, option) };
    if unsafe { wifexited(status) } {
        panic!("process exited before being killed");
    }
    ExitCode::SUCCESS
}

fn check_sigint_kills_running_daemon() -> ExitCode {
    let pid = unsafe { fork() };
    if pid == 0 {
        unsafe { taskmaster::main(Vec::with_capacity(0)) };
    }
    sleep(Duration::from_millis(400));

    unsafe {
        kill(pid as c_int, signal::Signal::SIGINT as c_int);
    }
    
    let status: c_int = 0;
    let option: c_int = 0;

    unsafe { waitpid(pid, &status, option) };
    if unsafe { wifexited(status) } {
        panic!("process exited before being killed");
    }

    ExitCode::SUCCESS
}