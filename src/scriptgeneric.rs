use crate::{Ptr, VoidPtr};
use angelscript_bindings::{
    asIScriptGeneric, asScriptGeneric_GetAddressOfArg, asScriptGeneric_GetAddressOfReturnLocation,
    asScriptGeneric_GetArgAddress, asScriptGeneric_GetArgObject, asScriptGeneric_GetObject,
    asScriptGeneric_SetReturnAddress, asScriptGeneric_SetReturnObject,
};

#[repr(C)]
pub struct ScriptGeneric {
    generic: *mut asIScriptGeneric,
}

impl ScriptGeneric {
    pub(crate) fn get_arg_dword(&self, p0: i32) -> u32 {
        todo!()
    }
}

impl ScriptGeneric {
    pub(crate) fn from_raw(generic: *mut asIScriptGeneric) -> Self {
        Self { generic }
    }
    pub(crate) fn as_ptr(&self) -> *mut asIScriptGeneric {
        self.generic
    }
    pub fn get_object<T>(&self) -> Ptr<T> {
        unsafe { Ptr::<T>::from_raw(asScriptGeneric_GetObject(self.as_ptr())) }
    }
    pub fn get_arg_object<T>(&self, idx: u32) -> Ptr<T> {
        unsafe { Ptr::<T>::from_raw(asScriptGeneric_GetArgObject(self.as_ptr(), idx)) }
    }
    pub fn get_arg_address<T>(&self, idx: u32) -> Ptr<T> {
        unsafe { Ptr::<T>::from_raw(asScriptGeneric_GetArgAddress(self.as_ptr(), idx)) }
    }
    pub fn get_address_of_arg<T>(&self, idx: u32) -> Ptr<T> {
        unsafe { Ptr::<T>::from_raw(asScriptGeneric_GetAddressOfArg(self.as_ptr(), idx)) }
    }

    pub fn get_address_of_return_location<T>(&self) -> Ptr<T> {
        unsafe { Ptr::<T>::from_raw(asScriptGeneric_GetAddressOfReturnLocation(self.as_ptr())) }
    }
    pub fn set_return_address(&self, ptr: &mut VoidPtr) {
        unsafe { asScriptGeneric_SetReturnAddress(self.as_ptr(), ptr.as_mut_ptr()) }
    }

    pub fn set_return_object(&self, ptr: &mut VoidPtr) {
        unsafe { asScriptGeneric_SetReturnObject(self.as_ptr(), ptr.as_mut_ptr()).into() }
    }
}
