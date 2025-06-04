use std::ffi::{c_char, CStr};

pub(crate) fn read_cstring(c_buf: *const c_char) -> &'static str {
    let c_str: &CStr = unsafe { CStr::from_ptr(c_buf) };
    c_str.to_str().unwrap()
}
