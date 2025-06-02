use crate::ffi::{asBOOL, asFALSE, asTRUE};
use std::ffi::{c_char, c_void, CStr};

// Helper functions
pub(crate) fn as_bool(value: bool) -> asBOOL {
    if value { asTRUE } else { asFALSE }
}

pub(crate) fn from_as_bool(value: asBOOL) -> bool {
    value != asFALSE
}

/// Trait for converting a `*mut c_void` to a reference of type `T`.
pub(crate) trait FromCVoidPtr {
    /// # Safety
    /// Caller must guarantee the pointer is valid, properly aligned, and no aliasing rules are violated.
    unsafe fn from_mut<'a>(ptr: *mut c_void) -> &'a mut Self;
    /// # Safety
    /// Caller must guarantee the pointer is valid, properly aligned, and no aliasing rules are violated.
    unsafe fn from_const<'a>(ptr: *mut c_void) -> &'a Self;
}

impl<T> FromCVoidPtr for T {
    unsafe fn from_mut<'a>(ptr: *mut c_void) -> &'a mut Self {
        &mut *(ptr as *mut T)
    }

    unsafe fn from_const<'a>(ptr: *mut c_void) -> &'a Self {
        &*(ptr as *const T)
    }
}

pub(crate) fn read_cstring(c_buf: *const c_char) -> &'static str {
    let c_str: &CStr = unsafe { CStr::from_ptr(c_buf) };
    c_str.to_str().unwrap()
}
