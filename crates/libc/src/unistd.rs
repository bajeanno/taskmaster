use std::ffi::c_int;
#[allow(deprecated)]
use std::os::unix::raw::mode_t as unix_mode_t;
#[allow(non_camel_case_types)]
#[allow(deprecated)]
pub type mode_t = unix_mode_t;

#[allow(non_camel_case_types)]
pub type size_t = usize;

#[allow(non_camel_case_types)]
pub type ssize_t = isize;

#[link(name = "c")]
unsafe extern "C" {
    pub fn fork() -> crate::sys::types::Pid;

    pub fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;

    pub fn umask(cmask: mode_t) -> mode_t;

    pub fn read(fd: c_int, buf: *mut core::ffi::c_void, count: size_t) -> ssize_t;

    pub fn write(fd: c_int, buf: *const core::ffi::c_void, count: size_t) -> ssize_t;
}
