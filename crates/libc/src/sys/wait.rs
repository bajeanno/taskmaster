use std::ffi::c_int;

unsafe extern "C" {
    pub fn waitpid(pid: c_int, stat_loc: &c_int, option: c_int) -> crate::sys::types::Pid;

    /// Linking with C to call preprocessor macro
    pub fn wifexited(status: c_int) -> bool;
}
