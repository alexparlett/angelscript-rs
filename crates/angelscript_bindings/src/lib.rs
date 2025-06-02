mod ffi {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

    unsafe impl Send for asIStringFactory {}
    unsafe impl Sync for asIStringFactory {}
}

pub use ffi::*;