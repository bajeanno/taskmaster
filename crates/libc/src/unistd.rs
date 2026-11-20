use std::ffi::{c_int, c_uint};
use std::os::raw::c_char;
#[allow(deprecated)]
use std::os::unix::raw::mode_t as unix_mode_t;

use crate::sys::types::{Gid, Uid};

#[allow(non_camel_case_types)]
#[allow(deprecated)]
pub type mode_t = unix_mode_t;

#[allow(non_camel_case_types)]
pub type size_t = usize;

#[allow(non_camel_case_types)]
pub type ssize_t = isize;

#[repr(C)]
pub struct GroupRaw {
    pub gr_name: *mut c_char,
    pub gr_passwd: *mut c_char,
    pub gr_gid: c_uint,
    pub gr_mem: *mut *mut c_char,
}

#[link(name = "c")]
unsafe extern "C" {
    pub fn fork() -> crate::sys::types::Pid;

    pub fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;

    pub fn umask(cmask: mode_t) -> mode_t;

    pub fn read(fd: c_int, buf: *mut core::ffi::c_void, count: size_t) -> ssize_t;

    pub fn write(fd: c_int, buf: *const core::ffi::c_void, count: size_t) -> ssize_t;

    pub fn getuid() -> Uid;

    pub fn getgid() -> Gid;

    pub fn setgid(gid: Gid) -> c_int;

    pub fn getgroups(gidsetsize: c_int, grouplist: *mut Gid) -> c_int;

    pub fn getgrnam_r(
        name: *const c_char,
        group: *mut GroupRaw,
        buffer: *mut c_char,
        buffer_len: usize,
        result: *mut *mut GroupRaw,
    ) -> c_int;
}
