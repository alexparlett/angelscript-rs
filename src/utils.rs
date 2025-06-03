use std::ffi::{c_char, c_void, CStr};

// Helper functions

/// Trait for converting a `*mut c_void` to a reference of type `T`.
pub(crate) trait FromCVoidPtr {
    /// # Safety
    /// Caller must guarantee the pointer is valid, properly aligned, and no aliasing rules are violated.
    fn from_mut<'a>(ptr: *mut c_void) -> &'a mut Self;
    /// # Safety
    /// Caller must guarantee the pointer is valid, properly aligned, and no aliasing rules are violated.
    fn from_const<'a>(ptr: *mut c_void) -> &'a Self;
}

impl<T> FromCVoidPtr for T {
    fn from_mut<'a>(ptr: *mut c_void) -> &'a mut Self {

        unsafe  { &mut *(ptr as *mut T) }
    }

    fn from_const<'a>(ptr: *mut c_void) -> &'a Self {
        unsafe  { &*(ptr as *const T) }
    }
}

pub(crate) fn read_cstring(c_buf: *const c_char) -> &'static str {
    let c_str: &CStr = unsafe { CStr::from_ptr(c_buf) };
    c_str.to_str().unwrap()
}