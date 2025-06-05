use crate::{Function, MessageType, TypeInfo, TypeModifiers};
use angelscript_bindings::{asDWORD, asJITFunction, asQWORD, asUINT};
use std::ffi::c_void;

#[derive(Hash, PartialEq, Eq, Copy, Clone, Debug)]
pub struct Ptr<T>(*mut T);

impl<T> Ptr<T> {
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }

    pub fn null() -> Self {
        Ptr(std::ptr::null_mut())
    }
    pub(crate) fn from(ptr: *mut T) -> Self {
        Ptr(ptr)
    }
    pub(crate) fn from_raw(ptr: *mut c_void) -> Self {
        Ptr(ptr as *mut T)
    }
    pub fn as_ptr(&self) -> *const T {
        self.0
    }
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.0
    }

    pub fn set(&mut self, value: T) {
        assert!(!self.is_null(), "Tried to access a null Ptr");
        assert_eq!(self.0 as usize % align_of::<T>(), 0, "Unaligned Ptr");

        unsafe {
            self.0.write(value);
        }
    }

    pub fn drop(&self) {
        assert!(!self.is_null(), "Tried to access a null Ptr");
        assert_eq!(self.0 as usize % align_of::<T>(), 0, "Unaligned Ptr");

        // unsafe { self.0.drop_in_place() };
    }

    pub fn as_void_ptr(&self) -> VoidPtr {
        VoidPtr(self.0 as *mut c_void)
    }

    pub fn read(&self) -> T {
        assert!(!self.is_null(), "Tried to access a null Ptr");
        assert_eq!(
            self.0 as usize % std::mem::align_of::<T>(),
            0,
            "Unaligned Ptr"
        );
        unsafe { self.0.read() }
    }

    pub fn as_ref(&self) -> &T {
        // Null pointer check
        assert!(!self.is_null(), "Tried to access a null Ptr");
        assert_eq!(
            self.0 as usize % std::mem::align_of::<T>(),
            0,
            "Unaligned Ptr"
        );

        unsafe { self.0.as_ref().unwrap() }
    }

    pub fn as_ref_mut(&mut self) -> &mut T {
        assert!(!self.is_null(), "Tried to access a null Ptr");
        assert_eq!(
            self.0 as usize % std::mem::align_of::<T>(),
            0,
            "Unaligned Ptr"
        );

        unsafe { self.0.as_mut().unwrap() }
    }
}

unsafe impl<T> Send for Ptr<T> {}
unsafe impl<T> Sync for Ptr<T> {}

#[derive(Hash, PartialEq, Eq, Copy, Clone, Debug)]
pub struct VoidPtr(*mut c_void);

impl VoidPtr {
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }

    pub fn null() -> Self {
        VoidPtr(std::ptr::null_mut())
    }
    pub fn from_mut_raw(ptr: *mut c_void) -> Self {
        VoidPtr(ptr)
    }
    pub fn from_const_raw(ptr: *const c_void) -> Self {
        VoidPtr(ptr as *mut c_void)
    }
    pub fn as_ptr(&self) -> *const c_void {
        self.0
    }
    pub fn as_mut_ptr(&mut self) -> *mut c_void {
        self.0
    }
}

impl<T> Into<VoidPtr> for *const T {
    fn into(self) -> VoidPtr {
        VoidPtr::from_mut_raw(self as *mut c_void)
    }
}

impl<T> Into<VoidPtr> for *mut T {
    fn into(self) -> VoidPtr {
        VoidPtr::from_mut_raw(self as *mut c_void)
    }
}

unsafe impl Send for VoidPtr {}
unsafe impl Sync for VoidPtr {}