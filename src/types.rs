use crate::{Function, MessageType, TypeInfo, TypeModifiers};
use angelscript_bindings::{asDWORD, asJITFunction, asQWORD, asUINT};
use std::ffi::c_void;

#[derive(Debug, Clone)]
pub struct VariableInfo {
    pub name: Option<String>,
    pub type_id: i32,
    pub type_modifiers: TypeModifiers,
    pub is_var_on_heap: bool,
    pub stack_offset: i32,
}

#[derive(Debug)]
pub struct StateRegisters {
    pub calling_system_function: Option<Function>,
    pub initial_function: Option<Function>,
    pub orig_stack_pointer: asDWORD,
    pub arguments_size: asDWORD,
    pub value_register: asQWORD,
    pub object_register: Option<Ptr<std::os::raw::c_void>>,
    pub object_type_register: Option<TypeInfo>,
}

#[derive(Debug)]
pub struct CallStateRegisters {
    pub stack_frame_pointer: asDWORD,
    pub current_function: Option<Function>,
    pub program_pointer: asDWORD,
    pub stack_pointer: asDWORD,
    pub stack_index: asDWORD,
}

#[derive(Debug)]
pub struct StackArgument<T> {
    pub type_id: i32,
    pub flags: asUINT,
    pub address: Option<Ptr<T>>,
}

#[derive(Debug)]
pub struct GlobalPropertyInfo {
    pub name: Option<String>,
    pub name_space: Option<String>,
    pub type_id: i32,
    pub is_const: bool,
    pub config_group: Option<String>,
    pub pointer: Ptr<std::os::raw::c_void>,
    pub access_mask: asDWORD,
}

#[derive(Debug)]
pub struct GCStatistics {
    pub current_size: asUINT,
    pub total_destroyed: asUINT,
    pub total_detected: asUINT,
    pub new_objects: asUINT,
    pub total_new_destroyed: asUINT,
}

#[derive(Debug)]
pub struct GCObjectInfo {
    pub seq_nbr: asUINT,
    pub obj: Ptr<std::os::raw::c_void>,
    pub type_info: Option<TypeInfo>,
}

#[derive(Debug)]
pub struct MessageInfo {
    pub section: String,
    pub row: u32,
    pub col: u32,
    pub msg_type: MessageType,
    pub message: String,
}

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
pub struct ModuleGlobalVarInfo {
    pub name: &'static str,
    pub namespace: &'static str,
    pub type_id: i32,
    pub is_const: bool,
}

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

pub trait ScriptEnum: Sized {
    fn get_value(&self) -> i32;

    fn from_value(value: i32) -> Self;
}

#[derive(Debug, Clone)]
pub struct DeclaredAtInfo {
    pub script_section: Option<&'static str>,
    pub row: i32,
    pub col: i32,
}

// Re-export JIT function type
pub type JITFunction = asJITFunction;

#[derive(Debug, Clone)]
pub struct PropertyInfo {
    pub index: asUINT,
    pub name: Option<String>,
    pub type_id: i32,
    pub address: VoidPtr,
}
