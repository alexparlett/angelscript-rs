use crate::types::script_memory::Void;

/// Trait for types that can be used in AngelScript registrations
pub trait ScriptData: Send + Sync {
    /// Get a pointer to the data for AngelScript
    fn to_script_ptr(&mut self) -> *mut Void;

    fn from_script_ptr(ptr: *mut Void) -> Self
    where
        Self: Sized;
}

/// Implement ScriptData for common types
impl<T: Sized + Send + Sync> ScriptData for T {
    fn to_script_ptr(&mut self) -> *mut Void {
        self as *mut T as *mut Void
    }

    fn from_script_ptr(ptr: *mut Void) -> Self {
        unsafe { (ptr as *mut T).read() }
    }
}