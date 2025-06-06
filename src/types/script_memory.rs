use std::ffi::c_void;
use crate::types::script_data::ScriptData;

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
    pub(crate) fn from_mut(ptr: *mut Void) -> Self {
        ScriptMemoryLocation(ptr)
    }
    pub(crate) fn from_const(ptr: *const Void) -> Self {
        ScriptMemoryLocation(ptr as *mut Void)
    }
    pub fn as_ptr(&self) -> *const Void {
        self.0
    }
    pub fn as_mut_ptr(&mut self) -> *mut Void {
        self.0
    }

    pub fn set<T>(&mut self, value: T) {
        assert!(!self.is_null(), "Tried to access a null Ptr");
        assert_eq!(self.0 as usize % align_of::<T>(), 0, "Unaligned Ptr");

        unsafe {
            self.0.cast::<T>().write(value);
        }
    }

    pub fn read<T: ScriptData>(&self) -> T {
        assert!(!self.is_null(), "Tried to access a null Ptr");
        assert_eq!(
            self.0 as usize % std::mem::align_of::<T>(),
            0,
            "Unaligned Ptr"
        );
        ScriptData::from_script_ptr(self.0)
    }

    pub fn as_ref<T>(&self) -> &T {
        // Null pointer check
        assert!(!self.is_null(), "Tried to access a null Ptr");
        assert_eq!(
            self.0 as usize % std::mem::align_of::<T>(),
            0,
            "Unaligned Ptr"
        );

        unsafe { self.0.cast::<T>().as_ref().unwrap() }
    }

    pub fn as_ref_mut<T>(&mut self) -> &mut T {
        assert!(!self.is_null(), "Tried to access a null Ptr");
        assert_eq!(
            self.0 as usize % std::mem::align_of::<T>(),
            0,
            "Unaligned Ptr"
        );

        unsafe { self.0.cast::<T>().as_mut().unwrap() }
    }
}

unsafe impl Send for ScriptMemoryLocation {}
unsafe impl Sync for ScriptMemoryLocation {}