use crate::error::{Result, Error};
use crate::typeinfo::TypeInfo;
use crate::module::Module;
use crate::types::*;
use crate::enums::*;
use std::ffi::{CStr, c_void};
use std::os::raw::c_char;
use std::ptr;
use std::marker::PhantomData;

pub struct Function {
    function: *mut asIScriptFunction,
    _phantom: PhantomData<asIScriptFunction>,
}

impl Function {
    pub(crate) fn from_raw(function: *mut asIScriptFunction) -> Self {
        Function {
            function,
            _phantom: PhantomData,
        }
    }

    pub fn get_engine(&self) -> *mut asIScriptEngine {
        unsafe {
            asFunction_GetEngine(self.function)
        }
    }

    pub fn add_ref(&self) -> Result<()> {
        unsafe {
            Error::from_code(asFunction_AddRef(self.function))
        }
    }

    pub fn release(&self) -> Result<()> {
        unsafe {
            Error::from_code(asFunction_Release(self.function))
        }
    }

    // Function info
    pub fn get_id(&self) -> i32 {
        unsafe {
            asFunction_GetId(self.function)
        }
    }

    pub fn get_func_type(&self) -> FuncType {
        unsafe {
            asFunction_GetFuncType(self.function)
        }
    }

    pub fn get_module_name(&self) -> Option<&str> {
        unsafe {
            let name = asFunction_GetModuleName(self.function);
            if name.is_null() {
                None
            } else {
                CStr::from_ptr(name).to_str().ok()
            }
        }
    }

    pub fn get_module(&self) -> Option<Module> {
        unsafe {
            let module = asFunction_GetModule(self.function);
            if module.is_null() {
                None
            } else {
                Some(Module::from_raw(module))
            }
        }
    }

    pub fn get_script_section_name(&self) -> Option<&str> {
        unsafe {
            let name = asFunction_GetScriptSectionName(self.function);
            if name.is_null() {
                None
            } else {
                CStr::from_ptr(name).to_str().ok()
            }
        }
    }

    pub fn get_config_group(&self) -> Option<&str> {
        unsafe {
            let group = asFunction_GetConfigGroup(self.function);
            if group.is_null() {
                None
            } else {
                CStr::from_ptr(group).to_str().ok()
            }
        }
    }

    pub fn get_access_mask(&self) -> asDWORD {
        unsafe {
            asFunction_GetAccessMask(self.function)
        }
    }

    pub fn get_auxiliary(&self) -> *mut c_void {
        unsafe {
            asFunction_GetAuxiliary(self.function)
        }
    }

    // Function signature
    pub fn get_object_type(&self) -> Option<TypeInfo> {
        unsafe {
            let type_info = asFunction_GetObjectType(self.function);
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    pub fn get_object_name(&self) -> Option<&str> {
        unsafe {
            let name = asFunction_GetObjectName(self.function);
            if name.is_null() {
                None
            } else {
                CStr::from_ptr(name).to_str().ok()
            }
        }
    }

    pub fn get_name(&self) -> Option<&str> {
        unsafe {
            let name = asFunction_GetName(self.function);
            if name.is_null() {
                None
            } else {
                CStr::from_ptr(name).to_str().ok()
            }
        }
    }

    pub fn get_namespace(&self) -> Option<&str> {
        unsafe {
            let namespace = asFunction_GetNamespace(self.function);
            if namespace.is_null() {
                None
            } else {
                CStr::from_ptr(namespace).to_str().ok()
            }
        }
    }

    pub fn get_declaration(&self, include_object_name: bool, include_namespace: bool, include_param_names: bool) -> Option<&str> {
        unsafe {
            let decl = asFunction_GetDeclaration(
                self.function,
                as_bool(include_object_name),
                as_bool(include_namespace),
                as_bool(include_param_names),
            );
            if decl.is_null() {
                None
            } else {
                CStr::from_ptr(decl).to_str().ok()
            }
        }
    }

    pub fn is_read_only(&self) -> bool {
        unsafe {
            from_as_bool(asFunction_IsReadOnly(self.function))
        }
    }

    pub fn is_private(&self) -> bool {
        unsafe {
            from_as_bool(asFunction_IsPrivate(self.function))
        }
    }

    pub fn is_protected(&self) -> bool {
        unsafe {
            from_as_bool(asFunction_IsProtected(self.function))
        }
    }

    pub fn is_final(&self) -> bool {
        unsafe {
            from_as_bool(asFunction_IsFinal(self.function))
        }
    }

    pub fn is_override(&self) -> bool {
        unsafe {
            from_as_bool(asFunction_IsOverride(self.function))
        }
    }

    pub fn is_shared(&self) -> bool {
        unsafe {
            from_as_bool(asFunction_IsShared(self.function))
        }
    }

    pub fn is_explicit(&self) -> bool {
        unsafe {
            from_as_bool(asFunction_IsExplicit(self.function))
        }
    }

    pub fn is_property(&self) -> bool {
        unsafe {
            from_as_bool(asFunction_IsProperty(self.function))
        }
    }

    // Parameters
    pub fn get_param_count(&self) -> asUINT {
        unsafe {
            asFunction_GetParamCount(self.function)
        }
    }

    pub fn get_param(&self, index: asUINT) -> Result<ParamInfo> {
        let mut type_id: i32 = 0;
        let mut flags: asDWORD = 0;
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
                name: if name.is_null() { None } else { CStr::from_ptr(name).to_str().ok() },
                default_arg: if default_arg.is_null() { None } else { CStr::from_ptr(default_arg).to_str().ok() },
            })
        }
    }

    // Return type
    pub fn get_return_type_id(&self) -> (i32, asDWORD) {
        let mut flags: asDWORD = 0;
        unsafe {
            let type_id = asFunction_GetReturnTypeId(self.function, &mut flags);
            (type_id, flags)
        }
    }

    // Type id for function pointers
    pub fn get_type_id(&self) -> i32 {
        unsafe {
            asFunction_GetTypeId(self.function)
        }
    }

    pub fn is_compatible_with_type_id(&self, type_id: i32) -> bool {
        unsafe {
            from_as_bool(asFunction_IsCompatibleWithTypeId(self.function, type_id))
        }
    }

    // Delegates
    pub fn get_delegate_object(&self) -> *mut c_void {
        unsafe {
            asFunction_GetDelegateObject(self.function)
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
    pub fn get_var_count(&self) -> asUINT {
        unsafe {
            asFunction_GetVarCount(self.function)
        }
    }

    pub fn get_var(&self, index: asUINT) -> Result<VarInfo> {
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
                name: if name.is_null() { None } else { CStr::from_ptr(name).to_str().ok() },
                type_id,
            })
        }
    }

    pub fn get_var_decl(&self, index: asUINT, include_namespace: bool) -> Option<&str> {
        unsafe {
            let decl = asFunction_GetVarDecl(self.function, index, as_bool(include_namespace));
            if decl.is_null() {
                None
            } else {
                CStr::from_ptr(decl).to_str().ok()
            }
        }
    }

    pub fn find_next_line_with_code(&self, line: i32) -> i32 {
        unsafe {
            asFunction_FindNextLineWithCode(self.function, line)
        }
    }

    // For JIT compilation
    pub fn get_byte_code(&self) -> Option<Vec<asDWORD>> {
        let mut length: asUINT = 0;
        unsafe {
            let byte_code = asFunction_GetByteCode(self.function, &mut length);
            if byte_code.is_null() || length == 0 {
                None
            } else {
                let slice = std::slice::from_raw_parts(byte_code, length as usize);
                Some(slice.to_vec())
            }
        }
    }

    // User data
    pub fn get_user_data(&self, type_: asPWORD) -> *mut c_void {
        unsafe {
            asFunction_GetUserData(self.function, type_)
        }
    }

    pub fn set_user_data(&self, data: *mut c_void, type_: asPWORD) -> *mut c_void {
        unsafe {
            asFunction_SetUserData(self.function, data, type_)
        }
    }

    pub(crate) fn as_ptr(&self) -> *mut asIScriptFunction {
        self.function
    }
}

#[derive(Debug, Clone)]
pub struct ParamInfo {
    pub type_id: i32,
    pub flags: asDWORD,
    pub name: Option<&'static str>,
    pub default_arg: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct VarInfo {
    pub name: Option<&'static str>,
    pub type_id: i32,
}

// Function doesn't need manual drop as it's reference counted
unsafe impl Send for Function {}
unsafe impl Sync for Function {}
