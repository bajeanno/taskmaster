use libc::unistd::mode_t;
use signal::Signal;

pub fn signal() -> Signal {
    Signal::SIGTERM
}

pub fn num_procs() -> u8 {
    1
}

pub fn work_dir() -> String {
    String::from("/")
}

pub fn exit_codes() -> Vec<u8> {
    vec![0]
}

pub fn umask() -> mode_t {
    0o022
}
