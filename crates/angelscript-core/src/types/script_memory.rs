use crate::types::script_data::ScriptData;
use std::ffi::c_void;

pub type Void = c_void;

#[derive(Hash, PartialEq, Eq, Copy, Clone, Debug)]
pub struct ScriptMemoryLocation(*mut Void);

impl ScriptMemoryLocation {
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }

    pub fn null() -> Self {
        ScriptMemoryLocation(std::ptr::null_mut())
    }
    /// Create a reference type by boxing the value and returning a handle to it
    pub fn from_boxed<T>(value: T) -> Self {
        let boxed = Box::new(value);
        let ptr = Box::into_raw(boxed);
        Self::from_const(ptr as *mut std::ffi::c_void)
    }

    /// Get a reference to the boxed value
    pub fn as_boxed_ref<T>(&self) -> &T {
        unsafe {
            let ptr = self.as_ptr() as *const T;
            &*ptr
        }
    }

    /// Get a mutable reference to the boxed value
    pub fn as_boxed_ref_mut<T>(&mut self) -> &mut T {
        unsafe {
            let ptr = self.as_ptr() as *mut T;
            &mut *ptr
        }
    }

    /// Release the boxed value (for use in Release behavior)
    /// Returns true if the object was actually freed
    pub unsafe fn release_boxed<T>(&self, ref_count: &std::sync::atomic::AtomicUsize) -> bool { unsafe {
        let count = ref_count.fetch_sub(1, std::sync::atomic::Ordering::Relaxed) - 1;

        if count == 0 {
            let ptr = self.as_ptr() as *mut T;
            let _boxed = Box::from_raw(ptr);
            true
        } else {
            false
        }
    }}

    /// Add reference to a boxed value
    pub fn addref_boxed(&self, ref_count: &std::sync::atomic::AtomicUsize) -> usize {
        ref_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1
    }
    
    pub fn from_mut(ptr: *mut Void) -> Self {
        ScriptMemoryLocation(ptr)
    }
    pub fn from_const(ptr: *const Void) -> Self {
        ScriptMemoryLocation(ptr as *mut Void)
    }
    pub fn as_ptr(&self) -> *const Void {
        self.0
    }
    pub fn as_mut_ptr(&mut self) -> *mut Void {
        self.0
    }

    pub fn set<T>(&mut self, value: T) {
        unsafe {
            self.0.cast::<T>().write(value);
        }
    }

    pub fn read<T: ScriptData>(&self) -> T {
        assert!(!self.is_null(), "Tried to access a null Ptr");
        ScriptData::from_script_ptr(self.0)
    }

    pub fn as_ref<T>(&self) -> &T {
        // Null pointer check
        assert!(!self.is_null(), "Tried to access a null Ptr");
        unsafe { self.0.cast::<T>().as_ref().unwrap() }
    }

    pub fn as_ref_mut<T>(&mut self) -> &mut T {
        assert!(!self.is_null(), "Tried to access a null Ptr");
        unsafe { self.0.cast::<T>().as_mut().unwrap() }
    }
}

unsafe impl Send for ScriptMemoryLocation {}
unsafe impl Sync for ScriptMemoryLocation {}
