use crate::enums::*;
use crate::error::{Error, Result};
use crate::ffi::{
    asFunction_AddRef, asFunction_FindNextLineWithCode, asFunction_GetAccessMask,
    asFunction_GetAuxiliary, asFunction_GetByteCode, asFunction_GetConfigGroup,
    asFunction_GetDeclaration, asFunction_GetDelegateFunction, asFunction_GetDelegateObject,
    asFunction_GetDelegateObjectType, asFunction_GetEngine, asFunction_GetFuncType,
    asFunction_GetId, asFunction_GetModule, asFunction_GetModuleName, asFunction_GetName,
    asFunction_GetNamespace, asFunction_GetObjectName, asFunction_GetObjectType,
    asFunction_GetParam, asFunction_GetParamCount, asFunction_GetReturnTypeId,
    asFunction_GetScriptSectionName, asFunction_GetTypeId, asFunction_GetUserData,
    asFunction_GetVar, asFunction_GetVarCount, asFunction_GetVarDecl,
    asFunction_IsCompatibleWithTypeId, asFunction_IsExplicit, asFunction_IsFinal,
    asFunction_IsOverride, asFunction_IsPrivate, asFunction_IsProperty, asFunction_IsProtected,
    asFunction_IsReadOnly, asFunction_IsShared, asFunction_Release, asFunction_SetUserData,
    asIScriptFunction,
};
use crate::module::Module;
use crate::typeinfo::TypeInfo;
use crate::types::*;
use crate::utils::FromCVoidPtr;
use crate::Engine;
use std::ffi::{c_void, CStr};
use std::os::raw::c_char;
use std::ptr;

pub struct Function {
    function: *mut asIScriptFunction,
}

impl Function {
    pub(crate) fn from_raw(function: *mut asIScriptFunction) -> Self {
        Function { function }
    }

    pub fn get_engine(&self) -> Engine {
        unsafe { Engine::from_raw(asFunction_GetEngine(self.function)) }
    }

    pub fn add_ref(&self) -> Result<()> {
        unsafe { Error::from_code(asFunction_AddRef(self.function)) }
    }

    pub fn release(&self) -> Result<()> {
        unsafe { Error::from_code(asFunction_Release(self.function)) }
    }

    // Function info
    pub fn get_id(&self) -> i32 {
        unsafe { asFunction_GetId(self.function) }
    }

    pub fn get_func_type(&self) -> FuncType {
        unsafe { asFunction_GetFuncType(self.function) }
    }

    pub fn get_module_name(&self) -> Result<&str> {
        unsafe {
            let name = asFunction_GetModuleName(self.function);
            if name.is_null() {
                Err(Error::NoModule)
            } else {
                CStr::from_ptr(name)
                    .to_str()
                    .map_err(|e| Error::Utf8Conversion(e))
            }
        }
    }

    pub fn get_module(&self) -> Result<Module> {
        unsafe {
            let module = asFunction_GetModule(self.function);
            if module.is_null() {
                Err(Error::NoModule)
            } else {
                Ok(Module::from_raw(module))
            }
        }
    }

    pub fn get_script_section_name(&self) -> Result<&str> {
        unsafe {
            let name = asFunction_GetScriptSectionName(self.function);
            if name.is_null() {
                Err(Error::NoSection)
            } else {
                CStr::from_ptr(name)
                    .to_str()
                    .map_err(|e| Error::Utf8Conversion(e))
            }
        }
    }

    pub fn get_config_group(&self) -> Result<&str> {
        unsafe {
            let group = asFunction_GetConfigGroup(self.function);
            if group.is_null() {
                Err(Error::NoConfigGroup)
            } else {
                CStr::from_ptr(group)
                    .to_str()
                    .map_err(|e| Error::Utf8Conversion(e))
            }
        }
    }

    pub fn get_access_mask(&self) -> u32 {
        unsafe { asFunction_GetAccessMask(self.function) }
    }

    pub fn get_auxiliary<'a, T>(&self) -> &'a mut T {
        unsafe {
            let ptr = asFunction_GetAuxiliary(self.function);
            T::from_mut(ptr)
        }
    }

    // Function signature
    pub fn get_object_type(&self) -> Result<TypeInfo> {
        unsafe {
            let type_info = asFunction_GetObjectType(self.function);
            if type_info.is_null() {
                Err(Error::InvalidObject)
            } else {
                Ok(TypeInfo::from_raw(type_info))
            }
        }
    }

    pub fn get_object_name(&self) -> Result<&str> {
        unsafe {
            let name = asFunction_GetObjectName(self.function);
            if name.is_null() {
                Err(Error::InvalidObject)
            } else {
                CStr::from_ptr(name)
                    .to_str()
                    .map_err(|e| Error::External(Box::new(e)))
            }
        }
    }

    pub fn get_name(&self) -> Result<&str> {
        unsafe {
            let name = asFunction_GetName(self.function);
            if name.is_null() {
                Err(Error::InvalidName)
            } else {
                CStr::from_ptr(name)
                    .to_str()
                    .map_err(|e| Error::External(Box::new(e)))
            }
        }
    }

    pub fn get_namespace(&self) -> Result<&str> {
        unsafe {
            let namespace = asFunction_GetNamespace(self.function);
            if namespace.is_null() {
                Err(Error::InvalidName)
            } else {
                CStr::from_ptr(namespace)
                    .to_str()
                    .map_err(|e| Error::Utf8Conversion(e))
            }
        }
    }

    pub fn get_declaration(
        &self,
        include_object_name: bool,
        include_namespace: bool,
        include_param_names: bool,
    ) -> Result<&str> {
        unsafe {
            let decl = asFunction_GetDeclaration(
                self.function,
                include_object_name,
                include_namespace,
                include_param_names,
            );
            if decl.is_null() {
                Err(Error::NoFunction)
            } else {
                CStr::from_ptr(decl)
                    .to_str()
                    .map_err(|e| Error::Utf8Conversion(e))
            }
        }
    }

    pub fn is_read_only(&self) -> bool {
        unsafe { asFunction_IsReadOnly(self.function) }
    }

    pub fn is_private(&self) -> bool {
        unsafe { asFunction_IsPrivate(self.function) }
    }

    pub fn is_protected(&self) -> bool {
        unsafe { asFunction_IsProtected(self.function) }
    }

    pub fn is_final(&self) -> bool {
        unsafe { asFunction_IsFinal(self.function) }
    }

    pub fn is_override(&self) -> bool {
        unsafe { asFunction_IsOverride(self.function) }
    }

    pub fn is_shared(&self) -> bool {
        unsafe { asFunction_IsShared(self.function) }
    }

    pub fn is_explicit(&self) -> bool {
        unsafe { asFunction_IsExplicit(self.function) }
    }

    pub fn is_property(&self) -> bool {
        unsafe { asFunction_IsProperty(self.function) }
    }

    // Parameters
    pub fn get_param_count(&self) -> u32 {
        unsafe { asFunction_GetParamCount(self.function) }
    }

    pub fn get_param(&self, index: u32) -> Result<ParamInfo> {
        let mut type_id: i32 = 0;
        let mut flags: u32 = 0;
        let mut name: *const c_char = ptr::null();
        let mut default_arg: *const c_char = ptr::null();

        unsafe {
            Error::from_code(asFunction_GetParam(
                self.function,
                index,
                &mut type_id,
                &mut flags,
                &mut name,
                &mut default_arg,
            ))?;

            Ok(ParamInfo {
                type_id,
                flags,
                name: if name.is_null() {
                    None
                } else {
                    CStr::from_ptr(name).to_str().ok()
                },
                default_arg: if default_arg.is_null() {
                    None
                } else {
                    CStr::from_ptr(default_arg).to_str().ok()
                },
            })
        }
    }

    // Return type
    pub fn get_return_type_id(&self) -> (i32, u32) {
        let mut flags: u32 = 0;
        unsafe {
            let type_id = asFunction_GetReturnTypeId(self.function, &mut flags);
            (type_id, flags)
        }
    }

    // Type id for function pointers
    pub fn get_type_id(&self) -> i32 {
        unsafe { asFunction_GetTypeId(self.function) }
    }

    pub fn is_compatible_with_type_id(&self, type_id: i32) -> bool {
        unsafe { asFunction_IsCompatibleWithTypeId(self.function, type_id) }
    }

    // Delegates
    pub fn get_delegate_object<'a, T>(&self) -> &'a mut T {
        unsafe {
            let ptr = asFunction_GetDelegateObject(self.function);
            T::from_mut(ptr)
        }
    }

    pub fn get_delegate_object_type(&self) -> Option<TypeInfo> {
        unsafe {
            let type_info = asFunction_GetDelegateObjectType(self.function);
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    pub fn get_delegate_function(&self) -> Option<Function> {
        unsafe {
            let func = asFunction_GetDelegateFunction(self.function);
            if func.is_null() {
                None
            } else {
                Some(Function::from_raw(func))
            }
        }
    }

    // Debug info
    pub fn get_var_count(&self) -> u32 {
        unsafe { asFunction_GetVarCount(self.function) }
    }

    pub fn get_var(&self, index: u32) -> Result<VarInfo> {
        let mut name: *const c_char = ptr::null();
        let mut type_id: i32 = 0;

        unsafe {
            Error::from_code(asFunction_GetVar(
                self.function,
                index,
                &mut name,
                &mut type_id,
            ))?;

            Ok(VarInfo {
                name: if name.is_null() {
                    None
                } else {
                    CStr::from_ptr(name).to_str().ok()
                },
                type_id,
            })
        }
    }

    pub fn get_var_decl(&self, index: u32, include_namespace: bool) -> Result<&str> {
        unsafe {
            let decl = asFunction_GetVarDecl(self.function, index, include_namespace);
            if decl.is_null() {
                Err(Error::InvalidName)
            } else {
                CStr::from_ptr(decl)
                    .to_str()
                    .map_err(|e| Error::Utf8Conversion(e))
            }
        }
    }

    pub fn find_next_line_with_code(&self, line: i32) -> i32 {
        unsafe { asFunction_FindNextLineWithCode(self.function, line) }
    }

    // For JIT compilation
    pub fn get_byte_code(&self) -> Result<Vec<u32>> {
        let mut length: u32 = 0;
        unsafe {
            let byte_code = asFunction_GetByteCode(self.function, &mut length);
            if byte_code.is_null() || length == 0 {
                Err(Error::NoFunction)
            } else {
                let slice = std::slice::from_raw_parts(byte_code, length as usize);
                Ok(slice.to_vec())
            }
        }
    }

    // User data
    pub fn get_user_data<'a, T: UserData>(&self) -> Result<&'a mut T> {
        unsafe {
            let ptr = asFunction_GetUserData(self.function, T::TypeId);
            if ptr.is_null() {
                return Err(Error::NullPointer);
            }
            Ok(T::from_mut(ptr))
        }
    }

    pub fn set_user_data<'a, T: UserData>(&self, data: &mut T) -> Option<&'a mut T> {
        unsafe {
            let ptr =
                asFunction_SetUserData(self.function, data as *mut _ as *mut c_void, T::TypeId);
            if ptr.is_null() {
                return None;
            }
            Some(T::from_mut(ptr))
        }
    }

    pub(crate) fn as_ptr(&self) -> *mut asIScriptFunction {
        self.function
    }
}

// Function doesn't need manual drop as it's reference counted
unsafe impl Send for Function {}
unsafe impl Sync for Function {}
