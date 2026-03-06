use std::ffi::c_int;
#[allow(deprecated)]
use std::os::unix::raw::mode_t;

#[link(name = "c")]
unsafe extern "C" {
    pub fn fork() -> crate::sys::types::Pid;

    pub fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;

    pub fn umask(cmask: mode_t) -> mode_t;
}
