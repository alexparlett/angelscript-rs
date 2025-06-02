#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
// Include the generated bindings
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));


// Re-export the raw bindings for advanced users
pub mod raw {
    pub use super::*;
}

// pub mod macros {
//     pub use angelscript_macros::*;
// }

// Public modules - aligned with wrapper files
mod types;
mod enums;
mod engine;
mod module;
mod context;
mod function;
mod typeinfo;
mod scriptobject;
mod error;

// Re-export main types
pub use types::*;
pub use enums::*;
pub use engine::*;
pub use module::*;
pub use context::*;
pub use function::*;
pub use typeinfo::*;
pub use scriptobject::*;
pub use error::{Result, Error};

// Core functions
use std::ffi::{c_char, CStr};

pub const VERSION: u32 = ANGELSCRIPT_VERSION;

pub fn create_script_engine() -> Result<Engine> {
    Engine::new()
}

pub(crate) fn read_cstring(c_buf: *const c_char) -> &'static str {

    let c_str: &CStr = unsafe { CStr::from_ptr(c_buf) };
    c_str.to_str().unwrap()
}

pub fn get_library_version() -> &'static str {
    unsafe {
        let version = asGetLibraryVersion();
        if version.is_null() {
            ""
        } else {
            CStr::from_ptr(version).to_str().unwrap_or("")
        }
    }
}

pub fn get_library_options() -> &'static str {
    unsafe {
        let options = asGetLibraryOptions();
        if options.is_null() {
            ""
        } else {
            CStr::from_ptr(options).to_str().unwrap_or("")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        let version = get_library_version();
        assert!(!version.is_empty());
        println!("AngelScript version: {}", version);
    }

    #[test]
    fn test_options() {
        let options = get_library_options();
        println!("AngelScript options: {}", options);
    }
}
