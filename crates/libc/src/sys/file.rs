pub const LOCK_SH: std::ffi::c_int = 1;
pub const LOCK_EX: std::ffi::c_int = 2;
pub const LOCK_UN: std::ffi::c_int = 8;
pub const LOCK_NB: std::ffi::c_int = 4;

#[link(name = "c")]
unsafe extern "C" {
    pub fn flock(fd: std::ffi::c_int, operation: std::ffi::c_int) -> std::ffi::c_int;
}
