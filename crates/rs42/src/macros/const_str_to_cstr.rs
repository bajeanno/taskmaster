/// Turns a const &str into a CStr
///
/// # Example
/// ```
/// use rs42::const_str_to_cstr;
/// use std::ffi::CStr;
///
/// const STR: &str = "42";
/// const CSTR: &CStr = const_str_to_cstr!(STR);
/// assert_eq!(CSTR, c"42");
/// ```
#[macro_export]
macro_rules! const_str_to_cstr {
    ($s:expr) => {{
        // Append null byte at compile time
        const SIZE: usize = $s.len() + 1;
        const BYTES: &[u8; SIZE] = &{
            let mut bytes = [0u8; SIZE];
            let mut i = 0;
            while i < $s.len() {
                bytes[i] = $s.as_bytes()[i];
                i += 1;
            }
            bytes[$s.len()] = 0;
            bytes
        };

        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(BYTES) }
    }};
}
