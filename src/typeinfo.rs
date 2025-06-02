use crate::error::{Error, Result};
use crate::ffi::{
    asBOOL, asEBehaviours, asFALSE, asITypeInfo, asTypeInfo_AddRef, asTypeInfo_DerivesFrom,
    asTypeInfo_GetAccessMask, asTypeInfo_GetBaseType, asTypeInfo_GetBehaviourByIndex,
    asTypeInfo_GetBehaviourCount, asTypeInfo_GetChildFuncdef, asTypeInfo_GetChildFuncdefCount,
    asTypeInfo_GetConfigGroup, asTypeInfo_GetEngine, asTypeInfo_GetEnumValueByIndex,
    asTypeInfo_GetEnumValueCount, asTypeInfo_GetFactoryByDecl, asTypeInfo_GetFactoryByIndex,
    asTypeInfo_GetFactoryCount, asTypeInfo_GetFlags, asTypeInfo_GetFuncdefSignature,
    asTypeInfo_GetInterface, asTypeInfo_GetInterfaceCount, asTypeInfo_GetMethodByDecl,
    asTypeInfo_GetMethodByIndex, asTypeInfo_GetMethodByName, asTypeInfo_GetMethodCount,
    asTypeInfo_GetModule, asTypeInfo_GetName, asTypeInfo_GetNamespace, asTypeInfo_GetParentType,
    asTypeInfo_GetProperty, asTypeInfo_GetPropertyCount, asTypeInfo_GetPropertyDeclaration,
    asTypeInfo_GetSize, asTypeInfo_GetSubType, asTypeInfo_GetSubTypeCount, asTypeInfo_GetSubTypeId,
    asTypeInfo_GetTypeId, asTypeInfo_GetTypedefTypeId, asTypeInfo_GetUserData,
    asTypeInfo_Implements, asTypeInfo_Release, asTypeInfo_SetUserData,
};
use crate::function::Function;
use crate::module::Module;
use crate::utils::{as_bool, from_as_bool, FromCVoidPtr};
use crate::{Engine, UserData};
use std::ffi::{c_void, CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use angelscript_bindings::{asContext_GetUserData, asContext_SetUserData};

pub struct TypeInfo {
    type_info: *mut asITypeInfo,
}

impl TypeInfo {
    pub(crate) fn from_raw(type_info: *mut asITypeInfo) -> Self {
        TypeInfo { type_info }
    }

    pub fn get_engine(&self) -> Engine {
        unsafe { Engine::from_raw(asTypeInfo_GetEngine(self.type_info)) }
    }

    pub fn get_config_group(&self) -> Result<&str> {
        unsafe {
            let group = asTypeInfo_GetConfigGroup(self.type_info);
            if group.is_null() {
                Err(Error::NullPointer)
            } else {
                CStr::from_ptr(group)
                    .to_str()
                    .map_err(|e| Error::Utf8Conversion(e))
            }
        }
    }

    pub fn get_access_mask(&self) -> u32 {
        unsafe { asTypeInfo_GetAccessMask(self.type_info) }
    }

    pub fn get_module(&self) -> Result<Module> {
        unsafe {
            let module = asTypeInfo_GetModule(self.type_info);
            if module.is_null() {
                Err(Error::NullPointer)
            } else {
                Ok(Module::from_raw(module))
            }
        }
    }

    pub fn add_ref(&self) -> Result<()> {
        unsafe { Error::from_code(asTypeInfo_AddRef(self.type_info)) }
    }

    pub fn release(&self) -> Result<()> {
        unsafe { Error::from_code(asTypeInfo_Release(self.type_info)) }
    }

    // Type info
    pub fn get_name(&self) -> Result<&str> {
        unsafe {
            let name = asTypeInfo_GetName(self.type_info);
            if name.is_null() {
                Err(Error::NullPointer)
            } else {
                CStr::from_ptr(name)
                    .to_str()
                    .map_err(|e| Error::Utf8Conversion(e))
            }
        }
    }

    pub fn get_namespace(&self) -> Result<&str> {
        unsafe {
            let namespace = asTypeInfo_GetNamespace(self.type_info);
            if namespace.is_null() {
                Err(Error::NullPointer)
            } else {
                CStr::from_ptr(namespace)
                    .to_str()
                    .map_err(|e| Error::Utf8Conversion(e))
            }
        }
    }

    pub fn get_base_type(&self) -> Result<TypeInfo> {
        unsafe {
            let base = asTypeInfo_GetBaseType(self.type_info);
            if base.is_null() {
                Err(Error::NullPointer)
            } else {
                Ok(TypeInfo::from_raw(base))
            }
        }
    }

    pub fn derives_from(&self, obj_type: &TypeInfo) -> bool {
        unsafe { from_as_bool(asTypeInfo_DerivesFrom(self.type_info, obj_type.type_info)) }
    }

    pub fn get_flags(&self) -> u32 {
        unsafe { asTypeInfo_GetFlags(self.type_info) }
    }

    pub fn get_size(&self) -> u32 {
        unsafe { asTypeInfo_GetSize(self.type_info) }
    }

    pub fn get_type_id(&self) -> i32 {
        unsafe { asTypeInfo_GetTypeId(self.type_info) }
    }

    pub fn get_sub_type_id(&self, sub_type_index: u32) -> i32 {
        unsafe { asTypeInfo_GetSubTypeId(self.type_info, sub_type_index) }
    }

    pub fn get_sub_type(&self, sub_type_index: u32) -> Result<TypeInfo> {
        unsafe {
            let sub_type = asTypeInfo_GetSubType(self.type_info, sub_type_index);
            if sub_type.is_null() {
                Err(Error::NullPointer)
            } else {
                Ok(TypeInfo::from_raw(sub_type))
            }
        }
    }

    pub fn get_sub_type_count(&self) -> u32 {
        unsafe { asTypeInfo_GetSubTypeCount(self.type_info) }
    }

    // Interfaces
    pub fn get_interface_count(&self) -> u32 {
        unsafe { asTypeInfo_GetInterfaceCount(self.type_info) }
    }

    pub fn get_interface(&self, index: u32) -> Result<TypeInfo> {
        unsafe {
            let interface = asTypeInfo_GetInterface(self.type_info, index);
            if interface.is_null() {
                Err(Error::NullPointer)
            } else {
                Ok(TypeInfo::from_raw(interface))
            }
        }
    }

    pub fn implements(&self, obj_type: &TypeInfo) -> bool {
        unsafe { from_as_bool(asTypeInfo_Implements(self.type_info, obj_type.type_info)) }
    }

    // Factories
    pub fn get_factory_count(&self) -> u32 {
        unsafe { asTypeInfo_GetFactoryCount(self.type_info) }
    }

    pub fn get_factory_by_index(&self, index: u32) -> Result<Function> {
        unsafe {
            let func = asTypeInfo_GetFactoryByIndex(self.type_info, index);
            if func.is_null() {
                Err(Error::NullPointer)
            } else {
                Ok(Function::from_raw(func))
            }
        }
    }

    pub fn get_factory_by_decl(&self, decl: &str) -> Result<Function> {
        let c_decl = CString::new(decl)?;

        unsafe {
            let func = asTypeInfo_GetFactoryByDecl(self.type_info, c_decl.as_ptr());
            if func.is_null() {
                Err(Error::NullPointer)
            } else {
                Ok(Function::from_raw(func))
            }
        }
    }

    // Methods
    pub fn get_method_count(&self) -> u32 {
        unsafe { asTypeInfo_GetMethodCount(self.type_info) }
    }

    pub fn get_method_by_index(&self, index: u32, get_virtual: bool) -> Result<Function> {
        unsafe {
            let func = asTypeInfo_GetMethodByIndex(self.type_info, index, as_bool(get_virtual));
            if func.is_null() {
                Err(Error::NullPointer)
            } else {
                Ok(Function::from_raw(func))
            }
        }
    }

    pub fn get_method_by_name(&self, name: &str, get_virtual: bool) -> Result<Function> {
        let c_name = CString::new(name)?;

        unsafe {
            let func =
                asTypeInfo_GetMethodByName(self.type_info, c_name.as_ptr(), as_bool(get_virtual));
            if func.is_null() {
                Err(Error::NullPointer)
            } else {
                Ok(Function::from_raw(func))
            }
        }
    }

    pub fn get_method_by_decl(&self, decl: &str, get_virtual: bool) -> Result<Function> {
        let c_decl = CString::new(decl)?;

        unsafe {
            let func =
                asTypeInfo_GetMethodByDecl(self.type_info, c_decl.as_ptr(), as_bool(get_virtual));
            if func.is_null() {
                Err(Error::NullPointer)
            } else {
                Ok(Function::from_raw(func))
            }
        }
    }

    // Properties
    pub fn get_property_count(&self) -> u32 {
        unsafe { asTypeInfo_GetPropertyCount(self.type_info) }
    }

    pub fn get_property(&self, index: u32) -> Result<TypePropertyInfo> {
        let mut name: *const c_char = ptr::null();
        let mut type_id: i32 = 0;
        let mut is_private: asBOOL = asFALSE;
        let mut is_protected: asBOOL = asFALSE;
        let mut offset: i32 = 0;
        let mut is_reference: asBOOL = asFALSE;
        let mut access_mask: u32 = 0;
        let mut composite_offset: i32 = 0;
        let mut is_composite_indirect: asBOOL = asFALSE;

        unsafe {
            Error::from_code(asTypeInfo_GetProperty(
                self.type_info,
                index,
                &mut name,
                &mut type_id,
                &mut is_private,
                &mut is_protected,
                &mut offset,
                &mut is_reference,
                &mut access_mask,
                &mut composite_offset,
                &mut is_composite_indirect,
            ))?;

            Ok(TypePropertyInfo {
                name: if name.is_null() {
                    None
                } else {
                    CStr::from_ptr(name).to_str().ok()
                },
                type_id,
                is_private: from_as_bool(is_private),
                is_protected: from_as_bool(is_protected),
                offset,
                is_reference: from_as_bool(is_reference),
                access_mask,
                composite_offset,
                is_composite_indirect: from_as_bool(is_composite_indirect),
            })
        }
    }

    pub fn get_property_declaration(&self, index: u32, include_namespace: bool) -> Result<&str> {
        unsafe {
            let decl = asTypeInfo_GetPropertyDeclaration(
                self.type_info,
                index,
                as_bool(include_namespace),
            );
            if decl.is_null() {
                Err(Error::NullPointer)
            } else {
                CStr::from_ptr(decl)
                    .to_str()
                    .map_err(|e| Error::Utf8Conversion(e))
            }
        }
    }

    // Behaviours
    pub fn get_behaviour_count(&self) -> u32 {
        unsafe { asTypeInfo_GetBehaviourCount(self.type_info) }
    }

    pub fn get_behaviour_by_index(
        &self,
        index: asEBehaviours,
    ) -> Result<(Function, asEBehaviours)> {
        let mut behaviour: asEBehaviours = index;

        unsafe {
            let func = asTypeInfo_GetBehaviourByIndex(self.type_info, index as u32, &mut behaviour);
            if func.is_null() {
                Err(Error::NullPointer)
            } else {
                Ok((Function::from_raw(func), behaviour))
            }
        }
    }

    // Child types
    pub fn get_child_funcdef_count(&self) -> u32 {
        unsafe { asTypeInfo_GetChildFuncdefCount(self.type_info) }
    }

    pub fn get_child_funcdef(&self, index: u32) -> Result<TypeInfo> {
        unsafe {
            let child = asTypeInfo_GetChildFuncdef(self.type_info, index);
            if child.is_null() {
                Err(Error::NullPointer)
            } else {
                Ok(TypeInfo::from_raw(child))
            }
        }
    }

    pub fn get_parent_type(&self) -> Result<TypeInfo> {
        unsafe {
            let parent = asTypeInfo_GetParentType(self.type_info);
            if parent.is_null() {
                Err(Error::NullPointer)
            } else {
                Ok(TypeInfo::from_raw(parent))
            }
        }
    }

    // Enums
    pub fn get_enum_value_count(&self) -> u32 {
        unsafe { asTypeInfo_GetEnumValueCount(self.type_info) }
    }

    pub fn get_enum_value_by_index(&self, index: u32) -> Result<(String, i32)> {
        let mut out_value: i32 = 0;

        unsafe {
            let name = asTypeInfo_GetEnumValueByIndex(self.type_info, index, &mut out_value);
            if name.is_null() {
                Err(Error::NullPointer)
            } else {
                CStr::from_ptr(name)
                    .to_str()
                    .map_err(|e| Error::Utf8Conversion(e))
                    .map(|s| (s.to_string(), out_value))
            }
        }
    }

    // Typedef
    pub fn get_typedef_type_id(&self) -> i32 {
        unsafe { asTypeInfo_GetTypedefTypeId(self.type_info) }
    }

    // Funcdef
    pub fn get_funcdef_signature(&self) -> Result<Function> {
        unsafe {
            let func = asTypeInfo_GetFuncdefSignature(self.type_info);
            if func.is_null() {
                Err(Error::NullPointer)
            } else {
                Ok(Function::from_raw(func))
            }
        }
    }

    // User data
    pub fn get_user_data<'a, T: UserData>(&self) -> Result<&'a mut T> {
        unsafe {
            let ptr = asTypeInfo_GetUserData(self.type_info, T::TypeId);
            if ptr.is_null() {
                return Err(Error::NullPointer)
            }
            Ok(T::from_mut(ptr))
        }
    }

    pub fn set_user_data<'a, T: UserData>(&self, data: &mut T) -> Option<&'a mut T> {
        unsafe {
            let ptr = asTypeInfo_SetUserData(self.type_info, data as *mut _ as *mut c_void, T::TypeId);
            if ptr.is_null() {
                return None
            }
            Some(T::from_mut(ptr))
        }
    }

    pub(crate) fn as_ptr(&self) -> *mut asITypeInfo {
        self.type_info
    }
}

#[derive(Debug, Clone)]
pub struct TypePropertyInfo {
    pub name: Option<&'static str>,
    pub type_id: i32,
    pub is_private: bool,
    pub is_protected: bool,
    pub offset: i32,
    pub is_reference: bool,
    pub access_mask: u32,
    pub composite_offset: i32,
    pub is_composite_indirect: bool,
}

// TypeInfo doesn't need manual drop as it's reference counted
unsafe impl Send for TypeInfo {}
unsafe impl Sync for TypeInfo {}
