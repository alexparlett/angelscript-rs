use crate::error::{Result, Error};
use crate::typeinfo::TypeInfo;
use crate::enums::*;
use std::ffi::CStr;
use std::os::raw::c_void;
use std::marker::PhantomData;

pub struct ScriptObject {
    object: *mut asIScriptObject,
    _phantom: PhantomData<asIScriptObject>,
}

impl ScriptObject {
    pub fn from_raw(object: *mut asIScriptObject) -> Self {
        ScriptObject {
            object,
            _phantom: PhantomData,
        }
    }

    pub fn get_engine(&self) -> *mut asIScriptEngine {
        unsafe {
            asScriptObject_GetEngine(self.object)
        }
    }

    pub fn add_ref(&self) -> Result<()> {
        unsafe {
            Error::from_code(asScriptObject_AddRef(self.object))
        }
    }

    pub fn release(&self) -> Result<()> {
        unsafe {
            Error::from_code(asScriptObject_Release(self.object))
        }
    }

    pub fn get_weak_ref_flag(&self) -> Option<*mut asILockableSharedBool> {
        unsafe {
            let flag = asScriptObject_GetWeakRefFlag(self.object);
            if flag.is_null() {
                None
            } else {
                Some(flag)
            }
        }
    }

    // Type info
    pub fn get_object_type(&self) -> Option<TypeInfo> {
        unsafe {
            let type_info = asScriptObject_GetObjectType(self.object);
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    // Properties
    pub fn get_property_count(&self) -> asUINT {
        unsafe {
            asScriptObject_GetPropertyCount(self.object)
        }
    }

    pub fn get_property_type_id(&self, prop: asUINT) -> i32 {
        unsafe {
            asScriptObject_GetPropertyTypeId(self.object, prop)
        }
    }

    pub fn get_property_name(&self, prop: asUINT) -> Option<&str> {
        unsafe {
            let name = asScriptObject_GetPropertyName(self.object, prop);
            if name.is_null() {
                None
            } else {
                CStr::from_ptr(name).to_str().ok()
            }
        }
    }

    pub fn get_address_of_property(&self, prop: asUINT) -> *mut c_void {
        unsafe {
            asScriptObject_GetAddressOfProperty(self.object, prop)
        }
    }

    // Object copying
    pub fn copy_from(&self, other: &ScriptObject) -> Result<()> {
        unsafe {
            Error::from_code(asScriptObject_CopyFrom(self.object, other.object))
        }
    }

    // User data
    pub fn get_user_data(&self, type_: asPWORD) -> *mut c_void {
        unsafe {
            asScriptObject_GetUserData(self.object, type_)
        }
    }

    pub fn set_user_data(&self, data: *mut c_void, type_: asPWORD) -> *mut c_void {
        unsafe {
            asScriptObject_SetUserData(self.object, data, type_)
        }
    }

    pub fn as_ptr(&self) -> *mut asIScriptObject {
        self.object
    }

    // Helper methods for property access
    pub fn get_property_value<T>(&self, prop: asUINT) -> Option<&T> {
        unsafe {
            let addr = self.get_address_of_property(prop);
            if addr.is_null() {
                None
            } else {
                Some(&*(addr as *const T))
            }
        }
    }

    pub fn get_property_value_mut<T>(&self, prop: asUINT) -> Option<&mut T> {
        unsafe {
            let addr = self.get_address_of_property(prop);
            if addr.is_null() {
                None
            } else {
                Some(&mut *(addr as *mut T))
            }
        }
    }

    pub fn set_property_value<T>(&self, prop: asUINT, value: T) -> Result<()> {
        unsafe {
            let addr = self.get_address_of_property(prop);
            if addr.is_null() {
                Err(Error::InvalidArg)
            } else {
                *(addr as *mut T) = value;
                Ok(())
            }
        }
    }

    // Convenience method to get property info
    pub fn get_property_info(&self, prop: asUINT) -> Option<PropertyInfo> {
        let name = self.get_property_name(prop)?;
        let type_id = self.get_property_type_id(prop);

        Some(PropertyInfo {
            index: prop,
            name: name.to_string(),
            type_id,
        })
    }

    // Get all properties
    pub fn get_all_properties(&self) -> Vec<PropertyInfo> {
        let count = self.get_property_count();
        let mut properties = Vec::with_capacity(count as usize);

        for i in 0..count {
            if let Some(info) = self.get_property_info(i) {
                properties.push(info);
            }
        }

        properties
    }
}

#[derive(Debug, Clone)]
pub struct PropertyInfo {
    pub index: asUINT,
    pub name: String,
    pub type_id: i32,
}

impl Drop for ScriptObject {
    fn drop(&mut self) {
        unsafe {
            asScriptObject_Release(self.object);
        }
    }
}

unsafe impl Send for ScriptObject {}
unsafe impl Sync for ScriptObject {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_property_info() {
        // This test would require a full engine setup
        // Just testing that the types compile correctly
        let _info = PropertyInfo {
            index: 0,
            name: "test".to_string(),
            type_id: 1,
        };
    }
}
