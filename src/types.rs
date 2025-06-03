use crate::ffi::asILockableSharedBool;
use crate::scriptgeneric::ScriptGeneric;
// Re-export basic types from raw bindings
use crate::MsgType;
use angelscript_bindings::asIScriptGeneric;
use std::ffi::c_void;
pub trait UserData {
    const TypeId: usize;
}

pub struct WeakRef(pub(crate) *mut asILockableSharedBool);

pub struct MessageInfo {
    pub section: String,
    pub row: u32,
    pub col: u32,
    pub msg_type: MsgType,
    pub message: String,
}

pub type MessageCallbackFn = fn(crate::MessageInfo);

#[derive(Debug, Clone)]
pub struct ParamInfo {
    pub type_id: i32,
    pub flags: u32,
    pub name: Option<&'static str>,
    pub default_arg: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct VarInfo {
    pub name: Option<&'static str>,
    pub type_id: i32,
}

#[derive(Debug, Clone)]
pub struct GlobalVarInfo {
    pub name: &'static str,
    pub namespace: &'static str,
    pub type_id: i32,
    pub is_const: bool,
}

pub(crate) type GenericFn = fn(&ScriptGeneric);

pub(crate) struct GenericFnUserData(pub GenericFn);

impl GenericFnUserData {
    pub fn call(&self, ctx: &ScriptGeneric) {
        (self.0)(ctx)
    }
}

impl UserData for GenericFnUserData {
    const TypeId: usize = 0x129032719; // Must be unique!
}

pub type ScriptGenericFn = unsafe extern "C" fn(ctx: *mut asIScriptGeneric);

#[derive(Hash, PartialEq, Eq, Copy, Clone, Debug)]
pub struct Ptr<T>(*mut T);

impl<T> Ptr<T> {
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }

    pub fn null() -> Self {
        Ptr(std::ptr::null_mut())
    }
    pub fn from_raw(ptr: *mut c_void) -> Self {
        Ptr(ptr as *mut T)
    }
    pub fn as_ptr(&self) -> *const T {
        self.0
    }
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.0
    }

    pub fn set(&mut self, value: T) {
        unsafe {
            *self.0 = value;
        }
    }

    pub fn as_void_ptr(&self) -> VoidPtr {
        VoidPtr(self.0 as *mut c_void)
    }

    pub fn as_ref(&self) -> &T {
        unsafe { &*self.0 }
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
