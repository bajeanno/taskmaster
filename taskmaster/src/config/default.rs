use libc::unistd::mode_t;
use signal::Signal;

pub fn default_signal() -> Signal {
    Signal::SIGTERM
}

pub fn default_num_procs() -> u8 {
    1
}

pub fn default_work_dir() -> String {
    String::from("/")
}

pub fn default_exit_codes() -> Vec<u8> {
    vec![0]
}

pub fn default_umask() -> mode_t {
    0o022
}
