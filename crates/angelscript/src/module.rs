use crate::error::{Result, Error};
use crate::function::Function;
use crate::typeinfo::TypeInfo;
use crate::context::Context;
use crate::types::*;
use crate::enums::*;
use std::ffi::{CString, CStr, c_void};
use std::os::raw::c_char;
use std::ptr;
use std::marker::PhantomData;

pub struct Module {
    module: *mut asIScriptModule,
    _phantom: PhantomData<asIScriptModule>,
}

impl Module {
    pub(crate) fn from_raw(module: *mut asIScriptModule) -> Self {
        Module {
            module,
            _phantom: PhantomData,
        }
    }

    pub fn get_engine(&self) -> *mut asIScriptEngine {
        unsafe {
            asModule_GetEngine(self.module)
        }
    }

    pub fn set_name(&self, name: &str) -> Result<()> {
        let c_name = CString::new(name)?;
        unsafe {
            asModule_SetName(self.module, c_name.as_ptr());
            Ok(())
        }
    }

    pub fn get_name(&self) -> &str {
        unsafe {
            let name = asModule_GetName(self.module);
            if name.is_null() {
                ""
            } else {
                CStr::from_ptr(name).to_str().unwrap_or("")
            }
        }
    }

    pub fn discard(&self) {
        unsafe {
            asModule_Discard(self.module);
        }
    }

    // Script sections
    pub fn add_script_section(&self, name: &str, code: &str) -> Result<()> {
        self.add_script_section_with_offset(name, code, 0)
    }

    pub fn add_script_section_with_offset(&self, name: &str, code: &str, line_offset: i32) -> Result<()> {
        let c_name = CString::new(name)?;
        let c_code = CString::new(code)?;

        unsafe {
            Error::from_code(asModule_AddScriptSection(
                self.module,
                c_name.as_ptr(),
                c_code.as_ptr(),
                code.len(),
                line_offset,
            ))
        }
    }

    // Build
    pub fn build(&self) -> Result<()> {
        unsafe {
            Error::from_code(asModule_Build(self.module))
        }
    }

    pub fn compile_function(&self, section_name: &str, code: &str, line_offset: i32, compile_flags: asDWORD) -> Result<Function> {
        let c_section = CString::new(section_name)?;
        let c_code = CString::new(code)?;
        let mut out_func: *mut asIScriptFunction = ptr::null_mut();

        unsafe {
            Error::from_code(asModule_CompileFunction(
                self.module,
                c_section.as_ptr(),
                c_code.as_ptr(),
                line_offset,
                compile_flags,
                &mut out_func,
            ))?;

            if out_func.is_null() {
                Err(Error::NoFunction)
            } else {
                Ok(Function::from_raw(out_func))
            }
        }
    }

    pub fn compile_global_var(&self, section_name: &str, code: &str, line_offset: i32) -> Result<()> {
        let c_section = CString::new(section_name)?;
        let c_code = CString::new(code)?;

        unsafe {
            Error::from_code(asModule_CompileGlobalVar(
                self.module,
                c_section.as_ptr(),
                c_code.as_ptr(),
                line_offset,
            ))
        }
    }

    // Namespaces
    pub fn set_default_namespace(&self, namespace: &str) -> Result<()> {
        let c_namespace = CString::new(namespace)?;

        unsafe {
            Error::from_code(asModule_SetDefaultNamespace(self.module, c_namespace.as_ptr()))
        }
    }

    pub fn get_default_namespace(&self) -> &str {
        unsafe {
            let namespace = asModule_GetDefaultNamespace(self.module);
            if namespace.is_null() {
                ""
            } else {
                CStr::from_ptr(namespace).to_str().unwrap_or("")
            }
        }
    }

    // Functions
    pub fn get_function_count(&self) -> asUINT {
        unsafe {
            asModule_GetFunctionCount(self.module)
        }
    }

    pub fn get_function_by_index(&self, index: asUINT) -> Option<Function> {
        unsafe {
            let func = asModule_GetFunctionByIndex(self.module, index);
            if func.is_null() {
                None
            } else {
                Some(Function::from_raw(func))
            }
        }
    }

    pub fn get_function_by_decl(&self, decl: &str) -> Result<Function> {
        let c_decl = CString::new(decl)?;

        unsafe {
            let func = asModule_GetFunctionByDecl(self.module, c_decl.as_ptr());
            if func.is_null() {
                Err(Error::NoFunction)
            } else {
                Ok(Function::from_raw(func))
            }
        }
    }

    pub fn get_function_by_name(&self, name: &str) -> Result<Function> {
        let c_name = CString::new(name)?;

        unsafe {
            let func = asModule_GetFunctionByName(self.module, c_name.as_ptr());
            if func.is_null() {
                Err(Error::NoFunction)
            } else {
                Ok(Function::from_raw(func))
            }
        }
    }

    pub fn remove_function(&self, func: &Function) -> Result<()> {
        unsafe {
            Error::from_code(asModule_RemoveFunction(self.module, func.as_ptr()))
        }
    }

    // Global variables
    pub fn reset_global_vars(&self, ctx: Option<&Context>) -> Result<()> {
        unsafe {
            let ctx_ptr = ctx.map(|c| c.as_ptr()).unwrap_or(ptr::null_mut());
            Error::from_code(asModule_ResetGlobalVars(self.module, ctx_ptr))
        }
    }

    pub fn get_global_var_count(&self) -> asUINT {
        unsafe {
            asModule_GetGlobalVarCount(self.module)
        }
    }

    pub fn get_global_var_index_by_name(&self, name: &str) -> Result<i32> {
        let c_name = CString::new(name)?;

        unsafe {
            let index = asModule_GetGlobalVarIndexByName(self.module, c_name.as_ptr());
            if index < 0 {
                Error::from_code(index)?;
            }
            Ok(index)
        }
    }

    pub fn get_global_var_index_by_decl(&self, decl: &str) -> Result<i32> {
        let c_decl = CString::new(decl)?;

        unsafe {
            let index = asModule_GetGlobalVarIndexByDecl(self.module, c_decl.as_ptr());
            if index < 0 {
                Error::from_code(index)?;
            }
            Ok(index)
        }
    }

    pub fn get_global_var_declaration(&self, index: asUINT, include_namespace: bool) -> &str {
        unsafe {
            let decl = asModule_GetGlobalVarDeclaration(self.module, index, as_bool(include_namespace));
            if decl.is_null() {
                ""
            } else {
                CStr::from_ptr(decl).to_str().unwrap_or("")
            }
        }
    }

    pub fn get_global_var(&self, index: asUINT) -> GlobalVarInfo {
        let mut name: *const c_char = ptr::null();
        let mut namespace: *const c_char = ptr::null();
        let mut type_id: i32 = 0;
        let mut is_const: asBOOL = asFALSE;

        unsafe {
            asModule_GetGlobalVar(self.module, index, &mut name, &mut namespace, &mut type_id, &mut is_const);

            GlobalVarInfo {
                name: if name.is_null() { "" } else { CStr::from_ptr(name).to_str().unwrap_or("") },
                namespace: if namespace.is_null() { "" } else { CStr::from_ptr(namespace).to_str().unwrap_or("") },
                type_id,
                is_const: from_as_bool(is_const),
            }
        }
    }

    pub fn get_address_of_global_var(&self, index: asUINT) -> *mut c_void {
        unsafe {
            asModule_GetAddressOfGlobalVar(self.module, index)
        }
    }

    pub fn remove_global_var(&self, index: asUINT) -> Result<()> {
        unsafe {
            Error::from_code(asModule_RemoveGlobalVar(self.module, index))
        }
    }

    // Type identification
    pub fn get_object_type_count(&self) -> asUINT {
        unsafe {
            asModule_GetObjectTypeCount(self.module)
        }
    }

    pub fn get_object_type_by_index(&self, index: asUINT) -> Option<TypeInfo> {
        unsafe {
            let type_info = asModule_GetObjectTypeByIndex(self.module, index);
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    pub fn get_type_id_by_decl(&self, decl: &str) -> Result<i32> {
        let c_decl = CString::new(decl)?;

        unsafe {
            let type_id = asModule_GetTypeIdByDecl(self.module, c_decl.as_ptr());
            if type_id < 0 {
                Error::from_code(type_id)?;
            }
            Ok(type_id)
        }
    }

    pub fn get_type_info_by_name(&self, name: &str) -> Option<TypeInfo> {
        let c_name = CString::new(name).ok()?;

        unsafe {
            let type_info = asModule_GetTypeInfoByName(self.module, c_name.as_ptr());
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    pub fn get_type_info_by_decl(&self, decl: &str) -> Option<TypeInfo> {
        let c_decl = CString::new(decl).ok()?;

        unsafe {
            let type_info = asModule_GetTypeInfoByDecl(self.module, c_decl.as_ptr());
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    pub fn as_ptr(&self) -> *mut asIScriptModule {
        self.module
    }
}

#[derive(Debug, Clone)]
pub struct GlobalVarInfo {
    pub name: &'static str,
    pub namespace: &'static str,
    pub type_id: i32,
    pub is_const: bool,
}

// Module doesn't need manual drop as it's managed by the engine
unsafe impl Send for Module {}
unsafe impl Sync for Module {}
