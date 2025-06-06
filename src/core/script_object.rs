use crate::core::engine::Engine;
use crate::core::error::{ScriptError, ScriptResult};
use crate::core::lockable_shared_bool::LockableSharedBool;
use crate::core::typeinfo::TypeInfo;
use crate::types::script_memory::ScriptMemoryLocation;
use crate::types::script_data::ScriptData;
use crate::types::user_data::UserData;
use angelscript_sys::{
    asIScriptEngine, asIScriptObject, asIScriptObject__bindgen_vtable, asPWORD, asUINT,
};
use std::ffi::CStr;
use std::ptr::NonNull;

/// Wrapper for AngelScript's script object interface
///
/// This represents an instance of a script class. It provides access to
/// the object's properties, type information, and reference counting.
#[derive(Debug, Clone)]
pub struct ScriptObject {
    inner: *mut asIScriptObject,
}

impl ScriptObject {
    /// Creates a ScriptObject wrapper from a raw pointer
    ///
    /// # Safety
    /// The pointer must be valid and point to a properly initialized asIScriptObject
    pub(crate) fn from_raw(ptr: *mut asIScriptObject) -> Self {
        let wrapper = Self { inner: ptr };
        wrapper
            .add_ref()
            .expect("Failed to add reference to script object");
        wrapper
    }

    /// Checks if the script object pointer is null
    pub fn is_null(&self) -> bool {
        self.inner.is_null()
    }

    // ========== VTABLE ORDER (matches asIScriptObject__bindgen_vtable) ==========

    // 1. AddRef
    pub fn add_ref(&self) -> ScriptResult<()> {
        unsafe { ScriptError::from_code((self.as_vtable().asIScriptObject_AddRef)(self.inner)) }
    }

    // 2. Release
    pub fn release(&self) -> ScriptResult<()> {
        unsafe { ScriptError::from_code((self.as_vtable().asIScriptObject_Release)(self.inner)) }
    }

    // 3. GetWeakRefFlag
    pub fn get_weak_ref_flag(&self) -> Option<LockableSharedBool> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptObject_GetWeakRefFlag)(self.inner);
            if ptr.is_null() {
                None
            } else {
                Some(LockableSharedBool::from_raw(ptr))
            }
        }
    }

    // 4. GetTypeId
    pub fn get_type_id(&self) -> i32 {
        unsafe { (self.as_vtable().asIScriptObject_GetTypeId)(self.inner) }
    }

    // 5. GetObjectType
    pub fn get_object_type(&self) -> TypeInfo {
        unsafe { TypeInfo::from_raw((self.as_vtable().asIScriptObject_GetObjectType)(self.inner)) }
    }

    // 6. GetPropertyCount
    pub fn get_property_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptObject_GetPropertyCount)(self.inner) }
    }

    // 7. GetPropertyTypeId
    pub fn get_property_type_id(&self, prop: asUINT) -> i32 {
        unsafe { (self.as_vtable().asIScriptObject_GetPropertyTypeId)(self.inner, prop) }
    }

    // 8. GetPropertyName
    pub fn get_property_name(&self, prop: asUINT) -> Option<&str> {
        unsafe {
            let name = (self.as_vtable().asIScriptObject_GetPropertyName)(self.inner, prop);
            if name.is_null() {
                None
            } else {
                CStr::from_ptr(name).to_str().ok()
            }
        }
    }

    // 9. GetAddressOfProperty
    pub fn get_address_of_property<T: ScriptData>(&self, prop: asUINT) -> Option<T> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptObject_GetAddressOfProperty)(self.inner, prop);
            if ptr.is_null() {
                None
            } else {
                Some(ScriptData::from_script_ptr(ptr))
            }
        }
    }

    // 10. GetEngine
    pub fn get_engine(&self) -> ScriptResult<Engine> {
        unsafe {
            let result: *mut asIScriptEngine =
                (self.as_vtable().asIScriptObject_GetEngine)(self.inner);
            let ptr = NonNull::new(result).ok_or(ScriptError::NullPointer)?;
            Ok(Engine::from_raw(NonNull::from(ptr)))
        }
    }

    // 11. CopyFrom
    pub fn copy_from(&self, other: &ScriptObject) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptObject_CopyFrom)(
                self.inner,
                other.inner,
            ))
        }
    }

    // 12. SetUserData
    pub fn set_user_data<T: UserData + ScriptData>(&self, data: &mut T) -> Option<T> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptObject_SetUserData)(
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

    // 13. GetUserData
    pub fn get_user_data<T: UserData + ScriptData>(&self) -> Option<T> {
        unsafe {
            let ptr =
                (self.as_vtable().asIScriptObject_GetUserData)(self.inner, T::TypeId as asPWORD);
            if ptr.is_null() {
                None
            } else {
                Some(ScriptData::from_script_ptr(ptr))
            }
        }
    }

    fn as_vtable(&self) -> &asIScriptObject__bindgen_vtable {
        unsafe { &*(*self.inner).vtable_ }
    }
}

impl Drop for ScriptObject {
    fn drop(&mut self) {
        self.release().expect("Failed to release script object");
    }
}

unsafe impl Send for ScriptObject {}
unsafe impl Sync for ScriptObject {}

// ========== CONVENIENCE METHODS ==========

impl ScriptObject {
    /// Gets a property value by index with type safety
    pub fn get_property<T: ScriptData>(&self, prop: asUINT) -> Option<T> {
        self.get_address_of_property::<T>(prop)
    }

    /// Sets a property value by index with type safety
    pub fn set_property<T: Sized>(&self, prop: asUINT, value: T) -> bool {
        unsafe {
            let ptr =
                (self.as_vtable().asIScriptObject_GetAddressOfProperty)(self.inner, prop) as *mut T;
            if ptr.is_null() {
                false
            } else {
                ptr.write(value);
                true
            }
        }
    }

    /// Gets a property by name
    pub fn get_property_by_name<T: ScriptData>(&self, name: &str) -> Option<T> {
        let prop_index = self.find_property_by_name(name)?;
        self.get_property::<T>(prop_index)
    }

    /// Sets a property by name
    pub fn set_property_by_name<T: ScriptData + Copy>(&self, name: &str, value: T) -> bool {
        if let Some(prop_index) = self.find_property_by_name(name) {
            self.set_property(prop_index, value)
        } else {
            false
        }
    }

    /// Finds a property index by name
    pub fn find_property_by_name(&self, name: &str) -> Option<asUINT> {
        let count = self.get_property_count();
        for i in 0..count {
            if let Some(prop_name) = self.get_property_name(i) {
                if prop_name == name {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Gets all properties as a vector of PropertyInfo
    pub fn get_all_properties(&self) -> Vec<PropertyInfo> {
        let count = self.get_property_count();
        (0..count)
            .map(|i| PropertyInfo {
                index: i,
                name: self.get_property_name(i).map(|s| s.to_string()),
                type_id: self.get_property_type_id(i),
                address: self
                    .get_address_of_property::<ScriptMemoryLocation>(i)
                    .unwrap_or(ScriptMemoryLocation::null()),
            })
            .collect()
    }

    /// Checks if this object is of a specific type
    pub fn is_type(&self, type_name: &str) -> bool {
        let type_info = self.get_object_type();
        if let Some(name) = type_info.get_name() {
            name == type_name
        } else {
            false
        }
    }

    /// Creates a weak reference to this object
    pub fn create_weak_ref(&self) -> Option<WeakScriptObjectRef> {
        let weak_flag = self.get_weak_ref_flag()?;
        Some(WeakScriptObjectRef {
            object_ptr: self.clone(),
            weak_flag,
        })
    }
}

// ========== ADDITIONAL TYPES ==========

/// Information about a script object property
#[derive(Debug, Clone)]
pub struct PropertyInfo {
    pub index: asUINT,
    pub name: Option<String>,
    pub type_id: i32,
    pub address: ScriptMemoryLocation,
}

/// Weak reference to a script object
///
/// This allows checking if the object is still alive without keeping it alive
#[derive(Debug)]
pub struct WeakScriptObjectRef {
    object_ptr: ScriptObject,
    weak_flag: LockableSharedBool,
}

impl WeakScriptObjectRef {
    /// Checks if the referenced object is still alive
    pub fn is_alive(&self) -> bool {
        !self.weak_flag.get()
    }

    /// Attempts to get a strong reference to the object
    ///
    /// Returns None if the object has been destroyed
    pub fn upgrade(&self) -> Option<ScriptObject> {
        if self.is_alive() {
            // Try to add a reference - this might fail if the object
            // is in the process of being destroyed
            unsafe {
                let vtable = self.object_ptr.as_vtable();
                if (vtable.asIScriptObject_AddRef)(self.object_ptr.inner) >= 0 {
                    Some(ScriptObject::from_raw(self.object_ptr.inner))
                } else {
                    None
                }
            }
        } else {
            None
        }
    }
}

unsafe impl Send for WeakScriptObjectRef {}
unsafe impl Sync for WeakScriptObjectRef {}
