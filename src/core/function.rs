use crate::core::engine::Engine;
use crate::types::enums::*;
use crate::core::error::{ScriptError, ScriptResult};
use crate::core::module::Module;
use crate::core::typeinfo::TypeInfo;
use crate::types::user_data::UserData;
use angelscript_sys::{asDWORD, asIScriptEngine, asIScriptFunction, asIScriptFunction__bindgen_vtable, asJITFunction, asPWORD, asUINT};
use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::ptr;
use std::ptr::NonNull;
use crate::types::script_memory::ScriptMemoryLocation;
use crate::types::script_data::ScriptData;

#[derive(Debug, Clone)]
pub struct Function {
    inner: *mut asIScriptFunction,
}

impl Function {
    pub(crate) fn from_raw(function: *mut asIScriptFunction) -> Self {
        let wrapper = Function { inner: function };
        wrapper
            .add_ref()
            .expect("Failed to add reference to function");
        wrapper
    }

    pub(crate) fn as_raw(&self) -> *mut asIScriptFunction {
        self.inner
    }

    // ========== VTABLE ORDER (matches asIScriptFunction__bindgen_vtable) ==========

    // 1. GetEngine
    pub fn get_engine(&self) -> ScriptResult<Engine> {
        unsafe {
            let result: *mut asIScriptEngine =
                (self.as_vtable().asIScriptFunction_GetEngine)(self.inner);
            let ptr = NonNull::new(result).ok_or(ScriptError::NullPointer)?;
            Ok(Engine::from_raw(NonNull::from(ptr)))
        }
    }
    // 2. AddRef
    pub fn add_ref(&self) -> ScriptResult<()> {
        unsafe { ScriptError::from_code((self.as_vtable().asIScriptFunction_AddRef)(self.inner)) }
    }

    // 3. Release
    pub fn release(&self) -> ScriptResult<()> {
        unsafe { ScriptError::from_code((self.as_vtable().asIScriptFunction_Release)(self.inner)) }
    }

    // 4. GetId
    pub fn get_id(&self) -> i32 {
        unsafe { (self.as_vtable().asIScriptFunction_GetId)(self.inner) }
    }

    // 5. GetFuncType
    pub fn get_func_type(&self) -> FunctionType {
        unsafe { FunctionType::from((self.as_vtable().asIScriptFunction_GetFuncType)(self.inner)) }
    }

    // 6. GetModuleName
    pub fn get_module_name(&self) -> Option<&str> {
        unsafe {
            let name = (self.as_vtable().asIScriptFunction_GetModuleName)(self.inner);
            if name.is_null() {
                None
            } else {
                CStr::from_ptr(name).to_str().ok()
            }
        }
    }

    // 7. GetModule
    pub fn get_module(&self) -> Option<Module> {
        unsafe {
            let module = (self.as_vtable().asIScriptFunction_GetModule)(self.inner);
            if module.is_null() {
                None
            } else {
                Some(Module::from_raw(module))
            }
        }
    }

    // 8. GetScriptSectionName
    pub fn get_script_section_name(&self) -> Option<&str> {
        unsafe {
            let name = (self.as_vtable().asIScriptFunction_GetScriptSectionName)(self.inner);
            if name.is_null() {
                None
            } else {
                CStr::from_ptr(name).to_str().ok()
            }
        }
    }

    // 9. GetConfigGroup
    pub fn get_config_group(&self) -> Option<&str> {
        unsafe {
            let group = (self.as_vtable().asIScriptFunction_GetConfigGroup)(self.inner);
            if group.is_null() {
                None
            } else {
                CStr::from_ptr(group).to_str().ok()
            }
        }
    }

    // 10. GetAccessMask
    pub fn get_access_mask(&self) -> asDWORD {
        unsafe { (self.as_vtable().asIScriptFunction_GetAccessMask)(self.inner) }
    }

    // 11. GetAuxiliary
    pub fn get_auxiliary<T: ScriptData>(&self) -> T {
        unsafe {
            let ptr = (self.as_vtable().asIScriptFunction_GetAuxiliary)(self.inner);
            ScriptData::from_script_ptr(ptr)
        }
    }

    // 12. GetObjectType
    pub fn get_object_type(&self) -> Option<TypeInfo> {
        unsafe {
            let type_info = (self.as_vtable().asIScriptFunction_GetObjectType)(self.inner);
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    // 13. GetObjectName
    pub fn get_object_name(&self) -> Option<&str> {
        unsafe {
            let name = (self.as_vtable().asIScriptFunction_GetObjectName)(self.inner);
            if name.is_null() {
                None
            } else {
                CStr::from_ptr(name).to_str().ok()
            }
        }
    }

    // 14. GetName
    pub fn get_name(&self) -> Option<&str> {
        unsafe {
            let name = (self.as_vtable().asIScriptFunction_GetName)(self.inner);
            if name.is_null() {
                None
            } else {
                CStr::from_ptr(name).to_str().ok()
            }
        }
    }

    // 15. GetNamespace
    pub fn get_namespace(&self) -> Option<&str> {
        unsafe {
            let namespace = (self.as_vtable().asIScriptFunction_GetNamespace)(self.inner);
            if namespace.is_null() {
                None
            } else {
                CStr::from_ptr(namespace).to_str().ok()
            }
        }
    }

    // 16. GetDeclaration
    pub fn get_declaration(
        &self,
        include_object_name: bool,
        include_namespace: bool,
        include_param_names: bool,
    ) -> ScriptResult<&str> {
        unsafe {
            let decl = (self.as_vtable().asIScriptFunction_GetDeclaration)(
                self.inner,
                include_object_name,
                include_namespace,
                include_param_names,
            );
            if decl.is_null() {
                Err(ScriptError::NullPointer)
            } else {
                CStr::from_ptr(decl)
                    .to_str()
                    .map_err(ScriptError::from)
            }
        }
    }

    // 17. IsReadOnly
    pub fn is_read_only(&self) -> bool {
        unsafe { (self.as_vtable().asIScriptFunction_IsReadOnly)(self.inner) }
    }

    // 18. IsPrivate
    pub fn is_private(&self) -> bool {
        unsafe { (self.as_vtable().asIScriptFunction_IsPrivate)(self.inner) }
    }

    // 19. IsProtected
    pub fn is_protected(&self) -> bool {
        unsafe { (self.as_vtable().asIScriptFunction_IsProtected)(self.inner) }
    }

    // 20. IsFinal
    pub fn is_final(&self) -> bool {
        unsafe { (self.as_vtable().asIScriptFunction_IsFinal)(self.inner) }
    }

    // 21. IsOverride
    pub fn is_override(&self) -> bool {
        unsafe { (self.as_vtable().asIScriptFunction_IsOverride)(self.inner) }
    }

    // 22. IsShared
    pub fn is_shared(&self) -> bool {
        unsafe { (self.as_vtable().asIScriptFunction_IsShared)(self.inner) }
    }

    // 23. IsExplicit
    pub fn is_explicit(&self) -> bool {
        unsafe { (self.as_vtable().asIScriptFunction_IsExplicit)(self.inner) }
    }

    // 24. IsProperty
    pub fn is_property(&self) -> bool {
        unsafe { (self.as_vtable().asIScriptFunction_IsProperty)(self.inner) }
    }

    // 25. GetParamCount
    pub fn get_param_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptFunction_GetParamCount)(self.inner) }
    }

    // 26. GetParam
    pub fn get_param(&self, index: asUINT) -> ScriptResult<ParamInfo> {
        let mut type_id: i32 = 0;
        let mut flags: asDWORD = 0;
        let mut name: *const c_char = ptr::null();
        let mut default_arg: *const c_char = ptr::null();

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptFunction_GetParam)(
                self.inner,
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

    // 27. GetReturnTypeId
    pub fn get_return_type_id(&self) -> (i32, asDWORD) {
        let mut flags: asDWORD = 0;
        unsafe {
            let type_id =
                (self.as_vtable().asIScriptFunction_GetReturnTypeId)(self.inner, &mut flags);
            (type_id, flags)
        }
    }

    // 28. GetTypeId
    pub fn get_type_id(&self) -> i32 {
        unsafe { (self.as_vtable().asIScriptFunction_GetTypeId)(self.inner) }
    }

    // 29. IsCompatibleWithTypeId
    pub fn is_compatible_with_type_id(&self, type_id: i32) -> bool {
        unsafe { (self.as_vtable().asIScriptFunction_IsCompatibleWithTypeId)(self.inner, type_id) }
    }

    // 30. GetDelegateObject
    pub fn get_delegate_object<T: ScriptData>(&self) -> T {
        unsafe {
            let ptr = (self.as_vtable().asIScriptFunction_GetDelegateObject)(self.inner);
            ScriptData::from_script_ptr(ptr)
        }
    }

    // 31. GetDelegateObjectType
    pub fn get_delegate_object_type(&self) -> Option<TypeInfo> {
        unsafe {
            let type_info = (self.as_vtable().asIScriptFunction_GetDelegateObjectType)(self.inner);
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    // 32. GetDelegateFunction
    pub fn get_delegate_function(&self) -> Option<Function> {
        unsafe {
            let func = (self.as_vtable().asIScriptFunction_GetDelegateFunction)(self.inner);
            if func.is_null() {
                None
            } else {
                Some(Function::from_raw(func))
            }
        }
    }

    // 33. GetVarCount
    pub fn get_var_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptFunction_GetVarCount)(self.inner) }
    }

    // 34. GetVar
    pub fn get_var(&self, index: asUINT) -> ScriptResult<VarInfo> {
        let mut name: *const c_char = ptr::null();
        let mut type_id: i32 = 0;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptFunction_GetVar)(
                self.inner,
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

    // 35. GetVarDecl
    pub fn get_var_decl(&self, index: asUINT, include_namespace: bool) -> ScriptResult<&str> {
        unsafe {
            let decl = (self.as_vtable().asIScriptFunction_GetVarDecl)(
                self.inner,
                index,
                include_namespace,
            );
            if decl.is_null() {
                Err(ScriptError::NullPointer)
            } else {
                CStr::from_ptr(decl)
                    .to_str()
                    .map_err(|e| ScriptError::Utf8Conversion(e))
            }
        }
    }

    // 36. FindNextLineWithCode
    pub fn find_next_line_with_code(&self, line: i32) -> ScriptResult<i32> {
        unsafe {
            let result =
                (self.as_vtable().asIScriptFunction_FindNextLineWithCode)(self.inner, line);
            if result < 0 {
                ScriptError::from_code(result)?;
            }
            Ok(result)
        }
    }

    // 37. GetDeclaredAt
    pub fn get_declared_at(&self) -> ScriptResult<DeclaredAtInfo> {
        let mut script_section: *const c_char = ptr::null();
        let mut row: i32 = 0;
        let mut col: i32 = 0;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptFunction_GetDeclaredAt)(
                self.inner,
                &mut script_section,
                &mut row,
                &mut col,
            ))?;

            Ok(DeclaredAtInfo {
                script_section: if script_section.is_null() {
                    None
                } else {
                    CStr::from_ptr(script_section).to_str().ok()
                },
                row,
                col,
            })
        }
    }

    // 38. GetByteCode
    pub fn get_byte_code(&self) -> Option<(ScriptMemoryLocation, asUINT)> {
        let mut length: asUINT = 0;
        unsafe {
            let ptr = (self.as_vtable().asIScriptFunction_GetByteCode)(self.inner, &mut length);
            if ptr.is_null() {
                None
            } else {
                Some((ScriptMemoryLocation::from_mut(ptr as *mut c_void), length))
            }
        }
    }

    // 39. SetJITFunction
    pub fn set_jit_function(&self, jit_func: asJITFunction) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptFunction_SetJITFunction)(
                self.inner, jit_func,
            ))
        }
    }

    // 40. GetJITFunction
    pub fn get_jit_function(&self) -> JITFunction {
        unsafe {
            let func = (self.as_vtable().asIScriptFunction_GetJITFunction)(self.inner);
            func
        }
    }

    // 41. SetUserData
    pub fn set_user_data<T: UserData + ScriptData>(&self, data: &mut T) -> Option<T> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptFunction_SetUserData)(
                self.inner,
                data.to_script_ptr(),
                T::TypeId as asPWORD,
            );
            if ptr.is_null() {
                None
            } else {
                Some(ScriptData::from_script_ptr(ptr))
            }
        }
    }

    // 42. GetUserData
    pub fn get_user_data<T: UserData + ScriptData>(&self) -> Option<T> {
        unsafe {
            let ptr =
                (self.as_vtable().asIScriptFunction_GetUserData)(self.inner, T::TypeId as asPWORD);
            if ptr.is_null() {
                None
            } else {
                Some(ScriptData::from_script_ptr(ptr))
            }
        }
    }

    fn as_vtable(&self) -> &asIScriptFunction__bindgen_vtable {
        unsafe { &*(*self.inner).vtable_ }
    }
}

impl Drop for Function {
    fn drop(&mut self) {
        self.release().expect("Failed to release function");
    }
}

unsafe impl Send for Function {}
unsafe impl Sync for Function {}

#[derive(Debug, Clone)]
pub struct DeclaredAtInfo {
    pub script_section: Option<&'static str>,
    pub row: i32,
    pub col: i32,
}

#[derive(Debug, Clone)]
pub struct VarInfo {
    pub name: Option<&'static str>,
    pub type_id: i32,
}

#[derive(Debug, Clone)]
pub struct ParamInfo {
    pub type_id: i32,
    pub flags: u32,
    pub name: Option<&'static str>,
    pub default_arg: Option<&'static str>,
}

// Re-export JIT function type
pub type JITFunction = asJITFunction;
