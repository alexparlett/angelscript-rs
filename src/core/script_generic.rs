use crate::core::engine::Engine;
use crate::core::function::Function;
use crate::types::script_memory::ScriptMemoryLocation;
use crate::types::script_data::ScriptData;
use crate::prelude::{FromScriptValue, ScriptArg, ScriptError, ScriptResult, TypeIdFlags};
use crate::types::script_value::ScriptValue;
use angelscript_sys::{
    asBYTE, asDWORD, asIScriptEngine, asIScriptGeneric, asIScriptGeneric__bindgen_vtable, asQWORD,
    asUINT, asWORD,
};
use std::ptr::NonNull;

/// Wrapper for AngelScript's generic interface
///
/// This interface is used when calling registered functions with the generic calling convention.
/// It provides access to function arguments, return values, and context information.
#[derive(Debug)]
pub struct ScriptGeneric {
    inner: *mut asIScriptGeneric,
}

impl ScriptGeneric {
    /// Creates a ScriptGeneric wrapper from a raw pointer
    ///
    /// # Safety
    /// The pointer must be valid and point to a properly initialized asIScriptGeneric
    pub(crate) fn from_raw(ptr: *mut asIScriptGeneric) -> Self {
        Self { inner: ptr }
    }

    // ========== VTABLE ORDER (matches asIScriptGeneric__bindgen_vtable) ==========

    // 1. GetEngine
    pub fn get_engine(&self) -> ScriptResult<Engine> {
        unsafe {
            let result: *mut asIScriptEngine =
                (self.as_vtable().asIScriptGeneric_GetEngine)(self.inner);
            let ptr = NonNull::new(result).ok_or(ScriptError::NullPointer)?;
            Ok(Engine::from_raw(NonNull::from(ptr)))
        }
    }

    // 2. GetFunction
    pub fn get_function(&self) -> Function {
        unsafe { Function::from_raw((self.as_vtable().asIScriptGeneric_GetFunction)(self.inner)) }
    }

    // 3. GetAuxiliary
    pub fn get_auxiliary<T: ScriptData>(&self) -> T {
        unsafe {
            let ptr = (self.as_vtable().asIScriptGeneric_GetAuxiliary)(self.inner);
            ScriptData::from_script_ptr(ptr)
        }
    }

    // 4. GetObject
    pub fn get_object(&self) -> Option<ScriptMemoryLocation> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptGeneric_GetObject)(self.inner);
            if ptr.is_null() {
                None
            } else {
                Some(ScriptMemoryLocation::from_mut(ptr))
            }
        }
    }

    // 5. GetObjectTypeId
    pub fn get_object_type_id(&self) -> i32 {
        unsafe { (self.as_vtable().asIScriptGeneric_GetObjectTypeId)(self.inner) }
    }

    // 6. GetArgCount
    pub fn get_arg_count(&self) -> i32 {
        unsafe { (self.as_vtable().asIScriptGeneric_GetArgCount)(self.inner) }
    }

    // 7. GetArgTypeId
    pub fn get_arg_type_id(&self, arg: asUINT) -> (i32, TypeIdFlags) {
        let mut flags: asDWORD = 0;
        unsafe {
            let type_id =
                (self.as_vtable().asIScriptGeneric_GetArgTypeId)(self.inner, arg, &mut flags);
            let typed_id_flags = TypeIdFlags::from_bits_truncate(flags);
            (type_id, typed_id_flags)
        }
    }

    // 8. GetArgByte
    pub fn get_arg_byte(&self, arg: asUINT) -> asBYTE {
        unsafe { (self.as_vtable().asIScriptGeneric_GetArgByte)(self.inner, arg) }
    }

    // 9. GetArgWord
    pub fn get_arg_word(&self, arg: asUINT) -> asWORD {
        unsafe { (self.as_vtable().asIScriptGeneric_GetArgWord)(self.inner, arg) }
    }

    // 10. GetArgDWord
    pub fn get_arg_dword(&self, arg: asUINT) -> asDWORD {
        unsafe { (self.as_vtable().asIScriptGeneric_GetArgDWord)(self.inner, arg) }
    }

    // 11. GetArgQWord
    pub fn get_arg_qword(&self, arg: asUINT) -> asQWORD {
        unsafe { (self.as_vtable().asIScriptGeneric_GetArgQWord)(self.inner, arg) }
    }

    // 12. GetArgFloat
    pub fn get_arg_float(&self, arg: asUINT) -> f32 {
        unsafe { (self.as_vtable().asIScriptGeneric_GetArgFloat)(self.inner, arg) }
    }

    // 13. GetArgDouble
    pub fn get_arg_double(&self, arg: asUINT) -> f64 {
        unsafe { (self.as_vtable().asIScriptGeneric_GetArgDouble)(self.inner, arg) }
    }

    // 14. GetArgAddress
    pub fn get_arg_address(&self, arg: asUINT) -> Option<ScriptMemoryLocation> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptGeneric_GetArgAddress)(self.inner, arg);
            if ptr.is_null() {
                None
            } else {
                Some(ScriptMemoryLocation::from_mut(ptr))
            }
        }
    }

    // 15. GetArgObject
    pub fn get_arg_object(&self, arg: asUINT) -> Option<ScriptMemoryLocation> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptGeneric_GetArgObject)(self.inner, arg);
            if ptr.is_null() {
                None
            } else {
                Some(ScriptMemoryLocation::from_mut(ptr))
            }
        }
    }

    // 16. GetAddressOfArg
    pub fn get_address_of_arg(&self, arg: asUINT) -> Option<ScriptMemoryLocation> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptGeneric_GetAddressOfArg)(self.inner, arg);
            if ptr.is_null() {
                None
            } else {
                Some(ScriptMemoryLocation::from_mut(ptr))
            }
        }
    }

    // 17. GetReturnTypeId
    pub fn get_return_type_id(&self) -> (i32, TypeIdFlags) {
        let mut flags: asDWORD = 0;
        unsafe {
            let type_id =
                (self.as_vtable().asIScriptGeneric_GetReturnTypeId)(self.inner, &mut flags);
            (type_id, TypeIdFlags::from_bits_truncate(flags))
        }
    }

    // 18. SetReturnByte
    pub fn set_return_byte(&self, val: asBYTE) -> crate::core::error::ScriptResult<()> {
        unsafe {
            crate::core::error::ScriptError::from_code((self
                .as_vtable()
                .asIScriptGeneric_SetReturnByte)(
                self.inner, val
            ))
        }
    }

    // 19. SetReturnWord
    pub fn set_return_word(&self, val: asWORD) -> crate::core::error::ScriptResult<()> {
        unsafe {
            crate::core::error::ScriptError::from_code((self
                .as_vtable()
                .asIScriptGeneric_SetReturnWord)(
                self.inner, val
            ))
        }
    }

    // 20. SetReturnDWord
    pub fn set_return_dword(&self, val: asDWORD) -> crate::core::error::ScriptResult<()> {
        unsafe {
            crate::core::error::ScriptError::from_code((self
                .as_vtable()
                .asIScriptGeneric_SetReturnDWord)(
                self.inner, val
            ))
        }
    }

    // 21. SetReturnQWord
    pub fn set_return_qword(&self, val: asQWORD) -> crate::core::error::ScriptResult<()> {
        unsafe {
            crate::core::error::ScriptError::from_code((self
                .as_vtable()
                .asIScriptGeneric_SetReturnQWord)(
                self.inner, val
            ))
        }
    }

    // 22. SetReturnFloat
    pub fn set_return_float(&self, val: f32) -> crate::core::error::ScriptResult<()> {
        unsafe {
            crate::core::error::ScriptError::from_code((self
                .as_vtable()
                .asIScriptGeneric_SetReturnFloat)(
                self.inner, val
            ))
        }
    }

    // 23. SetReturnDouble
    pub fn set_return_double(&self, val: f64) -> crate::core::error::ScriptResult<()> {
        unsafe {
            crate::core::error::ScriptError::from_code((self
                .as_vtable()
                .asIScriptGeneric_SetReturnDouble)(
                self.inner, val
            ))
        }
    }

    // 24. SetReturnAddress
    pub fn set_return_address_raw(
        &self,
        mut addr: ScriptMemoryLocation,
    ) -> crate::core::error::ScriptResult<()> {
        unsafe {
            crate::core::error::ScriptError::from_code((self
                .as_vtable()
                .asIScriptGeneric_SetReturnAddress)(
                self.inner, addr.as_mut_ptr()
            ))
        }
    }

    // 24. SetReturnAddress
    pub fn set_return_address<T: ScriptData>(
        &self,
        addr: &mut T,
    ) -> crate::core::error::ScriptResult<()> {
        unsafe {
            crate::core::error::ScriptError::from_code((self
                .as_vtable()
                .asIScriptGeneric_SetReturnAddress)(
                self.inner, addr.to_script_ptr()
            ))
        }
    }

    // 25. SetReturnObject
    pub fn set_return_object<T: ScriptData>(&self, obj: &mut T) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptGeneric_SetReturnObject)(
                self.inner,
                obj.to_script_ptr(),
            ))
        }
    }

    // 26. GetAddressOfReturnLocation
    pub fn get_address_of_return_location(&self) -> Option<ScriptMemoryLocation> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptGeneric_GetAddressOfReturnLocation)(self.inner);
            if ptr.is_null() {
                None
            } else {
                Some(ScriptMemoryLocation::from_mut(ptr))
            }
        }
    }

    /// Gets all arguments as a vector of GenericValue
    pub fn get_all_args(&self) -> Vec<ScriptArg> {
        let count = self.get_arg_count();
        (0..count as asUINT)
            .map(|i| {
                let (type_id, flags) = self.get_arg_type_id(i);
                ScriptArg {
                    type_id,
                    flags,
                    value: ScriptValue::from_generic(self, i, flags),
                }
            })
            .collect()
    }

    /// Checks if the function has a return value
    pub fn has_return_value(&self) -> bool {
        let (type_id, _) = self.get_return_type_id();
        type_id != 0 // Assuming 0 is void
    }

    /// Gets an argument with proper type checking
    pub fn get_arg_typed<T>(&self, arg: asUINT) -> Option<T>
    where
        T: FromScriptValue,
    {
        let (_, flags) = self.get_arg_type_id(arg);
        let value_data = ScriptValue::from_generic(self, arg, flags);
        T::from_script_value(&value_data)
    }

    fn as_vtable(&self) -> &asIScriptGeneric__bindgen_vtable {
        unsafe { &*(*self.inner).vtable_ }
    }
}

// ScriptGeneric doesn't manage its own lifetime
unsafe impl Send for ScriptGeneric {}
unsafe impl Sync for ScriptGeneric {}
