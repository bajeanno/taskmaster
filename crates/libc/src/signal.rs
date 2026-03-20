use std::ffi::c_int;

#[link(name = "c")]
unsafe extern "C" {
    pub fn kill(pid: crate::sys::types::Pid, sig: c_int) -> c_int;
}
