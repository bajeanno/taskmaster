use std::ffi::c_int;

#[link(name = "c")]
extern "C" {
    pub fn fork() -> crate::sys::types::Pid;

    pub fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
}
