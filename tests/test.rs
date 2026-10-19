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
}

fn cannot_start_after_crash() -> ExitCode {
    // TODO: create a pipe to bypass daemon start prompt
    let pid = unsafe { fork() };
    if pid == 0 {
        println!("starting");
        return unsafe {
            taskmaster::main(Some(vec!["./taskmaster".to_string(), "-y".to_string()]))
        };
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
    assert_eq!(unsafe { taskmaster::main(None) }, ExitCode::FAILURE);
    ExitCode::SUCCESS
}

fn force_start() -> ExitCode {
    let pid = unsafe { fork() };
    if pid == 0 {
        unsafe {
            taskmaster::main(Some(vec![
                "./taskmaster".to_string(),
                "--force".to_string(),
            ]))
        };
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
