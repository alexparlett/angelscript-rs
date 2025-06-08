use crate::core::context::Context;
use crate::core::engine::Engine;
use crate::core::error::{ScriptError, ScriptResult};
use crate::core::function::Function;
use crate::core::typeinfo::TypeInfo;
use crate::types::script_data::ScriptData;
use crate::types::user_data::UserData;
use angelscript_sys::{asDWORD, asIBinaryStream, asIScriptEngine, asIScriptFunction, asIScriptModule, asIScriptModule__bindgen_vtable, asUINT};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::ptr::NonNull;

#[derive(Debug, Clone)]
pub struct Module {
    inner: *mut asIScriptModule,
}

impl Module {
    pub(crate) fn from_raw(module: *mut asIScriptModule) -> Self {
        Self { inner: module }
    }

    // ========== VTABLE ORDER (matches asIScriptModule__bindgen_vtable) ==========

    // 1. GetEngine
    pub fn get_engine(&self) -> ScriptResult<Engine> {
        unsafe {
            let result: *mut asIScriptEngine =
                (self.as_vtable().asIScriptModule_GetEngine)(self.inner);
            let ptr = NonNull::new(result).ok_or(ScriptError::NullPointer)?;
            Ok(Engine::from_raw(ptr))
        }
    }

    // 2. SetName
    pub fn set_name(&self, name: &str) -> ScriptResult<()> {
        let c_name = CString::new(name)?;
        unsafe {
            (self.as_vtable().asIScriptModule_SetName)(self.inner, c_name.as_ptr());
        }
        Ok(())
    }

    // 3. GetName
    pub fn get_name(&self) -> Option<&str> {
        unsafe {
            let name = (self.as_vtable().asIScriptModule_GetName)(self.inner);
            if name.is_null() {
                None
            } else {
                CStr::from_ptr(name).to_str().ok()
            }
        }
    }

    // 4. Discard
    pub fn discard(&self) {
        unsafe {
            (self.as_vtable().asIScriptModule_Discard)(self.inner);
        }
    }

    // 5. AddScriptSection
    pub fn add_script_section(&self, name: &str, code: &str, line_offset: i32) -> ScriptResult<()> {
        let c_name = CString::new(name)?;
        let c_code = CString::new(code)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptModule_AddScriptSection)(
                self.inner,
                c_name.as_ptr(),
                c_code.as_ptr(),
                code.len(),
                line_offset,
            ))
        }
    }

    // 6. Build
    pub fn build(&self) -> ScriptResult<()> {
        unsafe { ScriptError::from_code((self.as_vtable().asIScriptModule_Build)(self.inner)) }
    }

    // 7. CompileFunction
    pub fn compile_function(
        &self,
        section_name: &str,
        code: &str,
        line_offset: i32,
        compile_flags: asDWORD,
    ) -> ScriptResult<Function> {
        let c_section_name = CString::new(section_name)?;
        let c_code = CString::new(code)?;
        let mut out_func: *mut asIScriptFunction = ptr::null_mut();

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptModule_CompileFunction)(
                self.inner,
                c_section_name.as_ptr(),
                c_code.as_ptr(),
                line_offset,
                compile_flags,
                &mut out_func,
            ))?;

            if out_func.is_null() {
                Err(ScriptError::NullPointer)
            } else {
                Ok(Function::from_raw(out_func))
            }
        }
    }

    // 8. CompileGlobalVar
    pub fn compile_global_var(
        &self,
        section_name: &str,
        code: &str,
        line_offset: i32,
    ) -> ScriptResult<()> {
        let c_section_name = CString::new(section_name)?;
        let c_code = CString::new(code)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptModule_CompileGlobalVar)(
                self.inner,
                c_section_name.as_ptr(),
                c_code.as_ptr(),
                line_offset,
            ))
        }
    }

    // 9. SetAccessMask
    pub fn set_access_mask(&self, access_mask: asDWORD) -> asDWORD {
        unsafe { (self.as_vtable().asIScriptModule_SetAccessMask)(self.inner, access_mask) }
    }

    // 10. SetDefaultNamespace
    pub fn set_default_namespace(&self, namespace: &str) -> ScriptResult<()> {
        let c_namespace = CString::new(namespace)?;
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptModule_SetDefaultNamespace)(
                self.inner,
                c_namespace.as_ptr(),
            ))
        }
    }

    // 11. GetDefaultNamespace
    pub fn get_default_namespace(&self) -> Option<&str> {
        unsafe {
            let namespace = (self.as_vtable().asIScriptModule_GetDefaultNamespace)(self.inner);
            if namespace.is_null() {
                None
            } else {
                CStr::from_ptr(namespace).to_str().ok()
            }
        }
    }

    // 12. GetFunctionCount
    pub fn get_function_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptModule_GetFunctionCount)(self.inner) }
    }

    // 13. GetFunctionByIndex
    pub fn get_function_by_index(&self, index: asUINT) -> Option<Function> {
        unsafe {
            let func = (self.as_vtable().asIScriptModule_GetFunctionByIndex)(self.inner, index);
            if func.is_null() {
                None
            } else {
                Some(Function::from_raw(func))
            }
        }
    }

    // 14. GetFunctionByDecl
    pub fn get_function_by_decl(&self, decl: &str) -> Option<Function> {
        let c_decl = match CString::new(decl) {
            Ok(s) => s,
            Err(_) => return None,
        };

        unsafe {
            let func =
                (self.as_vtable().asIScriptModule_GetFunctionByDecl)(self.inner, c_decl.as_ptr());
            if func.is_null() {
                None
            } else {
                Some(Function::from_raw(func))
            }
        }
    }

    // 15. GetFunctionByName
    pub fn get_function_by_name(&self, name: &str) -> Option<Function> {
        let c_name = match CString::new(name) {
            Ok(s) => s,
            Err(_) => return None,
        };

        unsafe {
            let func =
                (self.as_vtable().asIScriptModule_GetFunctionByName)(self.inner, c_name.as_ptr());
            if func.is_null() {
                None
            } else {
                Some(Function::from_raw(func))
            }
        }
    }

    // 16. RemoveFunction
    pub fn remove_function(&self, func: &Function) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptModule_RemoveFunction)(
                self.inner,
                func.as_raw(),
            ))
        }
    }

    // 17. ResetGlobalVars
    pub fn reset_global_vars(&self, ctx: Option<&Context>) -> ScriptResult<()> {
        unsafe {
            let ctx_ptr = match ctx {
                Some(context) => context.as_ptr(),
                None => ptr::null_mut(),
            };
            ScriptError::from_code((self.as_vtable().asIScriptModule_ResetGlobalVars)(
                self.inner, ctx_ptr,
            ))
        }
    }

    // 18. GetGlobalVarCount
    pub fn get_global_var_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptModule_GetGlobalVarCount)(self.inner) }
    }

    // 19. GetGlobalVarIndexByName
    pub fn get_global_var_index_by_name(&self, name: &str) -> Option<i32> {
        let c_name = match CString::new(name) {
            Ok(s) => s,
            Err(_) => return None,
        };

        unsafe {
            let index = (self.as_vtable().asIScriptModule_GetGlobalVarIndexByName)(
                self.inner,
                c_name.as_ptr(),
            );
            if index < 0 { None } else { Some(index) }
        }
    }

    // 20. GetGlobalVarIndexByDecl
    pub fn get_global_var_index_by_decl(&self, decl: &str) -> Option<i32> {
        let c_decl = match CString::new(decl) {
            Ok(s) => s,
            Err(_) => return None,
        };

        unsafe {
            let index = (self.as_vtable().asIScriptModule_GetGlobalVarIndexByDecl)(
                self.inner,
                c_decl.as_ptr(),
            );
            if index < 0 { None } else { Some(index) }
        }
    }

    // 21. GetGlobalVarDeclaration
    pub fn get_global_var_declaration(
        &self,
        index: asUINT,
        include_namespace: bool,
    ) -> Option<&str> {
        unsafe {
            let decl = (self.as_vtable().asIScriptModule_GetGlobalVarDeclaration)(
                self.inner,
                index,
                include_namespace,
            );
            if decl.is_null() {
                None
            } else {
                CStr::from_ptr(decl).to_str().ok()
            }
        }
    }

    // 22. GetGlobalVar
    pub fn get_global_var(&self, index: asUINT) -> ScriptResult<ModuleGlobalVarInfo> {
        let mut name: *const c_char = ptr::null();
        let mut namespace: *const c_char = ptr::null();
        let mut type_id: i32 = 0;
        let mut is_const: bool = false;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptModule_GetGlobalVar)(
                self.inner,
                index,
                &mut name,
                &mut namespace,
                &mut type_id,
                &mut is_const,
            ))?;

            Ok(ModuleGlobalVarInfo {
                name: if name.is_null() {
                    None
                } else {
                    CStr::from_ptr(name).to_str().ok().map(|s| s.to_string())
                },
                namespace: if namespace.is_null() {
                    None
                } else {
                    CStr::from_ptr(namespace)
                        .to_str()
                        .ok()
                        .map(|s| s.to_string())
                },
                type_id,
                is_const,
            })
        }
    }

    // 23. GetAddressOfGlobalVar
    pub fn get_address_of_global_var<T: ScriptData>(&self, index: asUINT) -> Option<T> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptModule_GetAddressOfGlobalVar)(self.inner, index);
            if ptr.is_null() {
                None
            } else {
                Some(ScriptData::from_script_ptr(ptr))
            }
        }
    }

    // 24. RemoveGlobalVar
    pub fn remove_global_var(&self, index: asUINT) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptModule_RemoveGlobalVar)(
                self.inner, index,
            ))
        }
    }

    // 25. GetObjectTypeCount
    pub fn get_object_type_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptModule_GetObjectTypeCount)(self.inner) }
    }

    // 26. GetObjectTypeByIndex
    pub fn get_object_type_by_index(&self, index: asUINT) -> Option<TypeInfo> {
        unsafe {
            let type_info =
                (self.as_vtable().asIScriptModule_GetObjectTypeByIndex)(self.inner, index);
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    // 27. GetTypeIdByDecl
    pub fn get_type_id_by_decl(&self, decl: &str) -> Option<i32> {
        let c_decl = match CString::new(decl) {
            Ok(s) => s,
            Err(_) => return None,
        };

        unsafe {
            let type_id =
                (self.as_vtable().asIScriptModule_GetTypeIdByDecl)(self.inner, c_decl.as_ptr());
            if type_id < 0 { None } else { Some(type_id) }
        }
    }

    // 28. GetTypeInfoByName
    pub fn get_type_info_by_name(&self, name: &str) -> Option<TypeInfo> {
        let c_name = match CString::new(name) {
            Ok(s) => s,
            Err(_) => return None,
        };

        unsafe {
            let type_info =
                (self.as_vtable().asIScriptModule_GetTypeInfoByName)(self.inner, c_name.as_ptr());
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    // 29. GetTypeInfoByDecl
    pub fn get_type_info_by_decl(&self, decl: &str) -> Option<TypeInfo> {
        let c_decl = match CString::new(decl) {
            Ok(s) => s,
            Err(_) => return None,
        };

        unsafe {
            let type_info =
                (self.as_vtable().asIScriptModule_GetTypeInfoByDecl)(self.inner, c_decl.as_ptr());
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    // 30. GetEnumCount
    pub fn get_enum_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptModule_GetEnumCount)(self.inner) }
    }

    // 31. GetEnumByIndex
    pub fn get_enum_by_index(&self, index: asUINT) -> Option<TypeInfo> {
        unsafe {
            let type_info = (self.as_vtable().asIScriptModule_GetEnumByIndex)(self.inner, index);
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    // 32. GetTypedefCount
    pub fn get_typedef_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptModule_GetTypedefCount)(self.inner) }
    }

    // 33. GetTypedefByIndex
    pub fn get_typedef_by_index(&self, index: asUINT) -> Option<TypeInfo> {
        unsafe {
            let type_info = (self.as_vtable().asIScriptModule_GetTypedefByIndex)(self.inner, index);
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    // 34. GetImportedFunctionCount
    pub fn get_imported_function_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptModule_GetImportedFunctionCount)(self.inner) }
    }

    // 35. GetImportedFunctionIndexByDecl
    pub fn get_imported_function_index_by_decl(&self, decl: &str) -> Option<i32> {
        let c_decl = match CString::new(decl) {
            Ok(s) => s,
            Err(_) => return None,
        };

        unsafe {
            let index = (self
                .as_vtable()
                .asIScriptModule_GetImportedFunctionIndexByDecl)(
                self.inner, c_decl.as_ptr()
            );
            if index < 0 { None } else { Some(index) }
        }
    }

    // 36. GetImportedFunctionDeclaration
    pub fn get_imported_function_declaration(&self, import_index: asUINT) -> Option<&str> {
        unsafe {
            let decl = (self
                .as_vtable()
                .asIScriptModule_GetImportedFunctionDeclaration)(
                self.inner, import_index
            );
            if decl.is_null() {
                None
            } else {
                CStr::from_ptr(decl).to_str().ok()
            }
        }
    }

    // 37. GetImportedFunctionSourceModule
    pub fn get_imported_function_source_module(&self, import_index: asUINT) -> Option<&str> {
        unsafe {
            let module = (self
                .as_vtable()
                .asIScriptModule_GetImportedFunctionSourceModule)(
                self.inner, import_index
            );
            if module.is_null() {
                None
            } else {
                CStr::from_ptr(module).to_str().ok()
            }
        }
    }

    // 38. BindImportedFunction
    pub fn bind_imported_function(&self, import_index: asUINT, func: &Function) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptModule_BindImportedFunction)(
                self.inner,
                import_index,
                func.as_raw(),
            ))
        }
    }

    // 39. UnbindImportedFunction
    pub fn unbind_imported_function(&self, import_index: asUINT) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptModule_UnbindImportedFunction)(
                self.inner,
                import_index,
            ))
        }
    }

    // 40. BindAllImportedFunctions
    pub fn bind_all_imported_functions(&self) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptModule_BindAllImportedFunctions)(
                self.inner,
            ))
        }
    }

    // 41. UnbindAllImportedFunctions
    pub fn unbind_all_imported_functions(&self) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self
                .as_vtable()
                .asIScriptModule_UnbindAllImportedFunctions)(
                self.inner
            ))
        }
    }

    // 42. SaveByteCode
    pub fn save_byte_code(&self, out: &mut BinaryStream, strip_debug_info: bool) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptModule_SaveByteCode)(
                self.inner,
                out.as_ptr(),
                strip_debug_info,
            ))
        }
    }

    // 43. LoadByteCode
    pub fn load_byte_code(&self, input: &mut BinaryStream) -> ScriptResult<bool> {
        let mut was_debug_info_stripped: bool = false;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptModule_LoadByteCode)(
                self.inner,
                input.as_ptr(),
                &mut was_debug_info_stripped,
            ))?;
        }

        Ok(was_debug_info_stripped)
    }

    // 44. SetUserData
    pub fn set_user_data<T: UserData + ScriptData>(&self, data: &mut T) -> Option<T> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptModule_SetUserData)(
                self.inner,
                data.to_script_ptr(),
                T::TYPE_ID,
            );
            if ptr.is_null() {
                None
            } else {
                Some(ScriptData::from_script_ptr(ptr))
            }
        }
    }

    // 45. GetUserData
    pub fn get_user_data<T: UserData + ScriptData>(&self) -> Option<T> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptModule_GetUserData)(self.inner, T::TYPE_ID);
            if ptr.is_null() {
                None
            } else {
                Some(ScriptData::from_script_ptr(ptr))
            }
        }
    }

    fn as_vtable(&self) -> &asIScriptModule__bindgen_vtable {
        unsafe { &*(*self.inner).vtable_ }
    }
}

// Module doesn't manage its own lifetime - the engine does
unsafe impl Send for Module {}
unsafe impl Sync for Module {}

// ========== ADDITIONAL TYPES ==========

#[derive(Debug, Clone)]
pub struct ModuleGlobalVarInfo {
    pub name: Option<String>,
    pub namespace: Option<String>,
    pub type_id: i32,
    pub is_const: bool,
}

/// Wrapper for binary stream operations
#[derive(Debug)]
pub struct BinaryStream {
    inner: *mut asIBinaryStream,
}

impl BinaryStream {

    pub(crate) fn as_ptr(&self) -> *mut asIBinaryStream {
        self.inner
    }
}

// ========== CONVENIENCE METHODS ==========

impl Module {
    /// Adds a script section with default line offset of 0
    pub fn add_script_section_simple(&self, name: &str, code: &str) -> ScriptResult<()> {
        self.add_script_section(name, code, 0)
    }

    /// Compiles a function with default flags
    pub fn compile_function_simple(&self, section_name: &str, code: &str) -> ScriptResult<Function> {
        self.compile_function(section_name, code, 0, 0)
    }

    /// Compiles a global variable with default line offset
    pub fn compile_global_var_simple(&self, section_name: &str, code: &str) -> ScriptResult<()> {
        self.compile_global_var(section_name, code, 0)
    }

    /// Gets all functions in the module
    pub fn get_all_functions(&self) -> Vec<Function> {
        let count = self.get_function_count();
        (0..count)
            .filter_map(|i| self.get_function_by_index(i))
            .collect()
    }

    /// Gets all global variables in the module
    pub fn get_all_global_vars(&self) -> Vec<ModuleGlobalVarInfo> {
        let count = self.get_global_var_count();
        (0..count)
            .filter_map(|i| self.get_global_var(i).ok())
            .collect()
    }

    /// Gets all object types in the module
    pub fn get_all_object_types(&self) -> Vec<TypeInfo> {
        let count = self.get_object_type_count();
        (0..count)
            .filter_map(|i| self.get_object_type_by_index(i))
            .collect()
    }

    /// Gets all enums in the module
    pub fn get_all_enums(&self) -> Vec<TypeInfo> {
        let count = self.get_enum_count();
        (0..count)
            .filter_map(|i| self.get_enum_by_index(i))
            .collect()
    }

    /// Gets all typedefs in the module
    pub fn get_all_typedefs(&self) -> Vec<TypeInfo> {
        let count = self.get_typedef_count();
        (0..count)
            .filter_map(|i| self.get_typedef_by_index(i))
            .collect()
    }
}
