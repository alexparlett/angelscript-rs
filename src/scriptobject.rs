use crate::error::{Error, Result};
use crate::ffi::{
    asIScriptObject, asScriptObject_AddRef, asScriptObject_CopyFrom,
    asScriptObject_GetAddressOfProperty, asScriptObject_GetEngine, asScriptObject_GetObjectType,
    asScriptObject_GetPropertyCount, asScriptObject_GetPropertyName,
    asScriptObject_GetPropertyTypeId, asScriptObject_GetUserData, asScriptObject_GetWeakRefFlag,
    asScriptObject_Release, asScriptObject_SetUserData,
};
use crate::typeinfo::TypeInfo;
use crate::utils::FromCVoidPtr;
use crate::{Engine, UserData, WeakRef};
use std::ffi::CStr;
use std::os::raw::c_void;

pub struct ScriptObject {
    object: *mut asIScriptObject,
}

impl ScriptObject {
    pub fn from_raw(object: *mut asIScriptObject) -> Self {
        ScriptObject { object }
    }

    pub fn get_engine(&self) -> Engine {
        unsafe { Engine::from_raw(asScriptObject_GetEngine(self.object)) }
    }

    pub fn add_ref(&self) -> Result<()> {
        unsafe { Error::from_code(asScriptObject_AddRef(self.object)) }
    }

    pub fn release(&self) -> Result<()> {
        unsafe { Error::from_code(asScriptObject_Release(self.object)) }
    }

    pub fn get_weak_ref_flag(&self) -> Result<WeakRef> {
        unsafe {
            let flag = asScriptObject_GetWeakRefFlag(self.object);
            if flag.is_null() {
                Err(Error::IllegalBehaviourForType)
            } else {
                Ok(WeakRef(flag))
            }
        }
    }

    // Type info
    pub fn get_object_type(&self) -> Result<TypeInfo> {
        unsafe {
            let type_info = asScriptObject_GetObjectType(self.object);
            if type_info.is_null() {
                Err(Error::InvalidObject)
            } else {
                Ok(TypeInfo::from_raw(type_info))
            }
        }
    }

    // Properties
    pub fn get_property_count(&self) -> u32 {
        unsafe { asScriptObject_GetPropertyCount(self.object) }
    }

    pub fn get_property_type_id(&self, prop: u32) -> i32 {
        unsafe { asScriptObject_GetPropertyTypeId(self.object, prop) }
    }

    pub fn get_property_name(&self, prop: u32) -> Result<&str> {
        unsafe {
            let name = asScriptObject_GetPropertyName(self.object, prop);
            if name.is_null() {
                Err(Error::InvalidArg)
            } else {
                CStr::from_ptr(name)
                    .to_str()
                    .map_err(|e| Error::Utf8Conversion(e))
            }
        }
    }

    pub fn get_address_of_property(&self, prop: u32) -> *mut c_void {
        unsafe { asScriptObject_GetAddressOfProperty(self.object, prop) }
    }

    // Object copying
    pub fn copy_from(&self, other: &ScriptObject) -> Result<()> {
        unsafe { Error::from_code(asScriptObject_CopyFrom(self.object, other.object)) }
    }

    // User data
    // User data
    pub fn get_user_data<'a, T: UserData>(&self) -> &'a mut T {
        unsafe {
            let ptr = asScriptObject_GetUserData(self.object, T::TypeId);
            T::from_mut(ptr)
        }
    }

    pub fn set_user_data<'a, T: UserData>(&self, data: &mut T) -> &'a mut T {
        unsafe {
            let ptr =
                asScriptObject_SetUserData(self.object, data as *mut _ as *mut c_void, T::TypeId);
            T::from_mut(ptr)
        }
    }

    pub fn as_ptr(&self) -> *mut asIScriptObject {
        self.object
    }

    // Helper methods for property access
    pub fn get_property_value<'a, T>(&self, prop: u32) -> Result<&'a T> {
        unsafe {
            let addr = self.get_address_of_property(prop);
            if addr.is_null() {
                Err(Error::NullPointer)
            } else {
                Ok(T::from_const(addr))
            }
        }
    }

    pub fn get_property_value_mut<'a, T>(&self, prop: u32) -> Result<&'a mut T> {
        unsafe {
            let addr = self.get_address_of_property(prop);
            if addr.is_null() {
                Err(Error::NullPointer)
            } else {
                Ok(T::from_mut(addr))
            }
        }
    }

    pub fn set_property_value<T>(&self, prop: u32, value: T) -> Result<()> {
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
    pub fn get_property_info(&self, prop: u32) -> Result<PropertyInfo> {
        let name = self.get_property_name(prop)?;
        let type_id = self.get_property_type_id(prop);

        Ok(PropertyInfo {
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
            if let Ok(info) = self.get_property_info(i) {
                properties.push(info);
            }
        }

        properties
    }
}

#[derive(Debug, Clone)]
pub struct PropertyInfo {
    pub index: u32,
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
