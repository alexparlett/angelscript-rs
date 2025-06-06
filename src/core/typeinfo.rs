use crate::core::engine::Engine;
use crate::types::enums::*;
use crate::core::error::{ScriptError, ScriptResult};
use crate::core::function::Function;
use crate::core::module::Module;
use crate::types::script_data::ScriptData;
use crate::types::user_data::UserData;
use angelscript_sys::{asDWORD, asEBehaviours, asEBehaviours_asBEHAVE_CONSTRUCT, asIScriptEngine, asITypeInfo, asITypeInfo__bindgen_vtable, asPWORD, asUINT};
use std::ffi::CStr;
use std::ptr::NonNull;

/// Wrapper for AngelScript's type information interface
///
/// This provides detailed information about types registered in AngelScript,
/// including classes, interfaces, enums, typedefs, and function definitions.
#[derive(Debug, Clone)]
pub struct TypeInfo {
    inner: *mut asITypeInfo,
}

impl TypeInfo {
    /// Creates a TypeInfo wrapper from a raw pointer
    ///
    /// # Safety
    /// The pointer must be valid and point to a properly initialized asITypeInfo
    pub(crate) fn from_raw(ptr: *mut asITypeInfo) -> Self {
        let wrapper = Self { inner: ptr };
        wrapper
            .add_ref()
            .expect("Failed to add reference to type info");
        wrapper
    }

    /// Returns the raw pointer to the type info
    pub(crate) fn as_ptr(&self) -> *mut asITypeInfo {
        self.inner
    }

    /// Checks if the type info pointer is null
    pub fn is_null(&self) -> bool {
        self.inner.is_null()
    }

    // ========== VTABLE ORDER (matches asITypeInfo__bindgen_vtable) ==========

    // 1. GetEngine
    pub fn get_engine(&self) -> ScriptResult<Engine> {
        unsafe {
            let result: *mut asIScriptEngine =
                (self.as_vtable().asITypeInfo_GetEngine)(self.inner);
            let ptr = NonNull::new(result).ok_or(ScriptError::NullPointer)?;
            Ok(Engine::from_raw(ptr))
        }
    }

    // 2. GetConfigGroup
    pub fn get_config_group(&self) -> Option<&str> {
        unsafe {
            let group = (self.as_vtable().asITypeInfo_GetConfigGroup)(self.inner);
            if group.is_null() {
                None
            } else {
                CStr::from_ptr(group).to_str().ok()
            }
        }
    }

    // 3. GetAccessMask
    pub fn get_access_mask(&self) -> asDWORD {
        unsafe { (self.as_vtable().asITypeInfo_GetAccessMask)(self.inner) }
    }

    // 4. GetModule
    pub fn get_module(&self) -> Option<Module> {
        unsafe {
            let module = (self.as_vtable().asITypeInfo_GetModule)(self.inner);
            if module.is_null() {
                None
            } else {
                Some(Module::from_raw(module))
            }
        }
    }

    // 5. AddRef
    pub fn add_ref(&self) -> ScriptResult<()> {
        unsafe { ScriptError::from_code((self.as_vtable().asITypeInfo_AddRef)(self.inner)) }
    }

    // 6. Release
    pub fn release(&self) -> ScriptResult<()> {
        unsafe { ScriptError::from_code((self.as_vtable().asITypeInfo_Release)(self.inner)) }
    }

    // 7. GetName
    pub fn get_name(&self) -> Option<&str> {
        unsafe {
            let name = (self.as_vtable().asITypeInfo_GetName)(self.inner);
            if name.is_null() {
                None
            } else {
                CStr::from_ptr(name).to_str().ok()
            }
        }
    }

    // 8. GetNamespace
    pub fn get_namespace(&self) -> Option<&str> {
        unsafe {
            let namespace = (self.as_vtable().asITypeInfo_GetNamespace)(self.inner);
            if namespace.is_null() {
                None
            } else {
                CStr::from_ptr(namespace).to_str().ok()
            }
        }
    }

    // 9. GetBaseType
    pub fn get_base_type(&self) -> Option<TypeInfo> {
        unsafe {
            let base_type = (self.as_vtable().asITypeInfo_GetBaseType)(self.inner);
            if base_type.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(base_type))
            }
        }
    }

    // 10. DerivesFrom
    pub fn derives_from(&self, obj_type: &TypeInfo) -> bool {
        unsafe { (self.as_vtable().asITypeInfo_DerivesFrom)(self.inner, obj_type.inner) }
    }

    // 11. GetFlags
    pub fn get_flags(&self) -> ObjectTypeFlags {
        unsafe { ((self.as_vtable().asITypeInfo_GetFlags)(self.inner)).into() }
    }

    // 12. GetSize
    pub fn get_size(&self) -> asUINT {
        unsafe { (self.as_vtable().asITypeInfo_GetSize)(self.inner) }
    }

    // 13. GetTypeId
    pub fn get_type_id(&self) -> i32 {
        unsafe { (self.as_vtable().asITypeInfo_GetTypeId)(self.inner) }
    }

    // 14. GetSubTypeId
    pub fn get_sub_type_id(&self, sub_type_index: asUINT) -> i32 {
        unsafe { (self.as_vtable().asITypeInfo_GetSubTypeId)(self.inner, sub_type_index) }
    }

    // 15. GetSubType
    pub fn get_sub_type(&self, sub_type_index: asUINT) -> Option<TypeInfo> {
        unsafe {
            let sub_type = (self.as_vtable().asITypeInfo_GetSubType)(self.inner, sub_type_index);
            if sub_type.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(sub_type))
            }
        }
    }

    // 16. GetSubTypeCount
    pub fn get_sub_type_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asITypeInfo_GetSubTypeCount)(self.inner) }
    }

    // 17. GetInterfaceCount
    pub fn get_interface_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asITypeInfo_GetInterfaceCount)(self.inner) }
    }

    // 18. GetInterface
    pub fn get_interface(&self, index: asUINT) -> Option<TypeInfo> {
        unsafe {
            let interface = (self.as_vtable().asITypeInfo_GetInterface)(self.inner, index);
            if interface.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(interface))
            }
        }
    }

    // 19. Implements
    pub fn implements(&self, obj_type: &TypeInfo) -> bool {
        unsafe { (self.as_vtable().asITypeInfo_Implements)(self.inner, obj_type.inner) }
    }

    // 20. GetFactoryCount
    pub fn get_factory_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asITypeInfo_GetFactoryCount)(self.inner) }
    }

    // 21. GetFactoryByIndex
    pub fn get_factory_by_index(&self, index: asUINT) -> Option<Function> {
        unsafe {
            let factory = (self.as_vtable().asITypeInfo_GetFactoryByIndex)(self.inner, index);
            if factory.is_null() {
                None
            } else {
                Some(Function::from_raw(factory))
            }
        }
    }

    // 22. GetFactoryByDecl
    pub fn get_factory_by_decl(&self, decl: &str) -> Option<Function> {
        let c_decl = match std::ffi::CString::new(decl) {
            Ok(s) => s,
            Err(_) => return None,
        };

        unsafe {
            let factory =
                (self.as_vtable().asITypeInfo_GetFactoryByDecl)(self.inner, c_decl.as_ptr());
            if factory.is_null() {
                None
            } else {
                Some(Function::from_raw(factory))
            }
        }
    }

    // 23. GetMethodCount
    pub fn get_method_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asITypeInfo_GetMethodCount)(self.inner) }
    }

    // 24. GetMethodByIndex
    pub fn get_method_by_index(&self, index: asUINT, get_virtual: bool) -> Option<Function> {
        unsafe {
            let method =
                (self.as_vtable().asITypeInfo_GetMethodByIndex)(self.inner, index, get_virtual);
            if method.is_null() {
                None
            } else {
                Some(Function::from_raw(method))
            }
        }
    }

    // 25. GetMethodByName
    pub fn get_method_by_name(&self, name: &str, get_virtual: bool) -> Option<Function> {
        let c_name = match std::ffi::CString::new(name) {
            Ok(s) => s,
            Err(_) => return None,
        };

        unsafe {
            let method = (self.as_vtable().asITypeInfo_GetMethodByName)(
                self.inner,
                c_name.as_ptr(),
                get_virtual,
            );
            if method.is_null() {
                None
            } else {
                Some(Function::from_raw(method))
            }
        }
    }

    // 26. GetMethodByDecl
    pub fn get_method_by_decl(&self, decl: &str, get_virtual: bool) -> Option<Function> {
        let c_decl = match std::ffi::CString::new(decl) {
            Ok(s) => s,
            Err(_) => return None,
        };

        unsafe {
            let method = (self.as_vtable().asITypeInfo_GetMethodByDecl)(
                self.inner,
                c_decl.as_ptr(),
                get_virtual,
            );
            if method.is_null() {
                None
            } else {
                Some(Function::from_raw(method))
            }
        }
    }

    // 27. GetPropertyCount
    pub fn get_property_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asITypeInfo_GetPropertyCount)(self.inner) }
    }

    // 28. GetProperty
    pub fn get_property(&self, index: asUINT) -> ScriptResult<TypePropertyInfo> {
        let mut name: *const std::os::raw::c_char = std::ptr::null();
        let mut type_id: i32 = 0;
        let mut is_private: bool = false;
        let mut is_protected: bool = false;
        let mut offset: i32 = 0;
        let mut is_reference: bool = false;
        let mut access_mask: asDWORD = 0;
        let mut composite_offset: i32 = 0;
        let mut is_composite_indirect: bool = false;

        unsafe {
            ScriptError::from_code((self.as_vtable().asITypeInfo_GetProperty)(
                self.inner,
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
                    CStr::from_ptr(name).to_str().ok().map(|s| s.to_string())
                },
                type_id,
                is_private,
                is_protected,
                offset,
                is_reference,
                access_mask,
                composite_offset,
                is_composite_indirect,
            })
        }
    }

    // 29. GetPropertyDeclaration
    pub fn get_property_declaration(&self, index: asUINT, include_namespace: bool) -> Option<&str> {
        unsafe {
            let decl = (self.as_vtable().asITypeInfo_GetPropertyDeclaration)(
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

    // 30. GetBehaviourCount
    pub fn get_behaviour_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asITypeInfo_GetBehaviourCount)(self.inner) }
    }

    // 31. GetBehaviourByIndex
    pub fn get_behaviour_by_index(&self, index: asUINT) -> Option<(Function, Behaviour)> {
        let mut out_behaviour: asEBehaviours = asEBehaviours_asBEHAVE_CONSTRUCT;

        unsafe {
            let func = (self.as_vtable().asITypeInfo_GetBehaviourByIndex)(
                self.inner,
                index,
                &mut out_behaviour,
            );
            if func.is_null() {
                None
            } else {
                Some((Function::from_raw(func), out_behaviour.into()))
            }
        }
    }

    // 32. GetChildFuncdefCount
    pub fn get_child_funcdef_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asITypeInfo_GetChildFuncdefCount)(self.inner) }
    }

    // 33. GetChildFuncdef
    pub fn get_child_funcdef(&self, index: asUINT) -> Option<TypeInfo> {
        unsafe {
            let funcdef = (self.as_vtable().asITypeInfo_GetChildFuncdef)(self.inner, index);
            if funcdef.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(funcdef))
            }
        }
    }

    // 34. GetParentType
    pub fn get_parent_type(&self) -> Option<TypeInfo> {
        unsafe {
            let parent = (self.as_vtable().asITypeInfo_GetParentType)(self.inner);
            if parent.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(parent))
            }
        }
    }

    // 35. GetEnumValueCount
    pub fn get_enum_value_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asITypeInfo_GetEnumValueCount)(self.inner) }
    }

    // 36. GetEnumValueByIndex
    pub fn get_enum_value_by_index(&self, index: asUINT) -> Option<(String, i32)> {
        let mut out_value: i32 = 0;

        unsafe {
            let name = (self.as_vtable().asITypeInfo_GetEnumValueByIndex)(
                self.inner,
                index,
                &mut out_value,
            );
            if name.is_null() {
                None
            } else {
                CStr::from_ptr(name)
                    .to_str()
                    .ok()
                    .map(|s| (s.to_string(), out_value))
            }
        }
    }

    // 37. GetTypedefTypeId
    pub fn get_typedef_type_id(&self) -> i32 {
        unsafe { (self.as_vtable().asITypeInfo_GetTypedefTypeId)(self.inner) }
    }

    // 38. GetFuncdefSignature
    pub fn get_funcdef_signature(&self) -> Option<Function> {
        unsafe {
            let signature = (self.as_vtable().asITypeInfo_GetFuncdefSignature)(self.inner);
            if signature.is_null() {
                None
            } else {
                Some(Function::from_raw(signature))
            }
        }
    }

    // 39. SetUserData
    pub fn set_user_data<T: UserData + ScriptData>(&self, data: &mut T) -> Option<T> {
        unsafe {
            let ptr = (self.as_vtable().asITypeInfo_SetUserData)(
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

    // 40. GetUserData
    pub fn get_user_data<T: UserData + ScriptData>(&self) -> Option<T> {
        unsafe {
            let ptr = (self.as_vtable().asITypeInfo_GetUserData)(self.inner, T::TypeId as asPWORD);
            if ptr.is_null() {
                None
            } else {
                Some(ScriptData::from_script_ptr(ptr))
            }
        }
    }

    fn as_vtable(&self) -> &asITypeInfo__bindgen_vtable {
        unsafe { &*(*self.inner).vtable_ }
    }
}

impl Drop for TypeInfo {
    fn drop(&mut self) {
        self.release().expect("Failed to release type info");
    }
}

unsafe impl Send for TypeInfo {}
unsafe impl Sync for TypeInfo {}

// ========== CONVENIENCE METHODS ==========

impl TypeInfo {
    /// Gets the full name including namespace
    pub fn get_full_name(&self) -> String {
        match (self.get_namespace(), self.get_name()) {
            (Some(ns), Some(name)) if !ns.is_empty() => format!("{}::{}", ns, name),
            (_, Some(name)) => name.to_string(),
            _ => "<unknown>".to_string(),
        }
    }

    /// Checks if this type is a specific type by name
    pub fn is_type(&self, type_name: &str) -> bool {
        self.get_name() == Some(type_name)
    }

    /// Checks if this type is a class
    pub fn is_class(&self) -> bool {
        let flags = self.get_flags();
        (flags & ObjectTypeFlags::REF).is_empty() || (flags & ObjectTypeFlags::VALUE).is_empty()
    }

    /// Checks if this type is an interface
    pub fn is_interface(&self) -> bool {
        // Interfaces typically have specific flags or can be determined by having no size
        self.get_size() == 0 && self.get_method_count() > 0
    }

    /// Checks if this type is an enum
    pub fn is_enum(&self) -> bool {
        let flags = self.get_flags();
        (flags & ObjectTypeFlags::ENUM).is_empty()
    }

    /// Checks if this type is a typedef
    pub fn is_typedef(&self) -> bool {
        let flags = self.get_flags();
        (flags & ObjectTypeFlags::TYPEDEF).is_empty()
    }

    /// Checks if this type is a function definition
    pub fn is_funcdef(&self) -> bool {
        let flags = self.get_flags();
        (flags & ObjectTypeFlags::FUNCDEF).is_empty()
    }

    /// Gets all methods as a vector
    pub fn get_all_methods(&self, get_virtual: bool) -> Vec<Function> {
        let count = self.get_method_count();
        (0..count)
            .filter_map(|i| self.get_method_by_index(i, get_virtual))
            .collect()
    }

    /// Gets all factories as a vector
    pub fn get_all_factories(&self) -> Vec<Function> {
        let count = self.get_factory_count();
        (0..count)
            .filter_map(|i| self.get_factory_by_index(i))
            .collect()
    }

    /// Gets all properties as a vector
    pub fn get_all_properties(&self) -> Vec<TypePropertyInfo> {
        let count = self.get_property_count();
        (0..count)
            .filter_map(|i| self.get_property(i).ok())
            .collect()
    }

    /// Gets all interfaces as a vector
    pub fn get_all_interfaces(&self) -> Vec<TypeInfo> {
        let count = self.get_interface_count();
        (0..count).filter_map(|i| self.get_interface(i)).collect()
    }

    /// Gets all sub-types as a vector
    pub fn get_all_sub_types(&self) -> Vec<TypeInfo> {
        let count = self.get_sub_type_count();
        (0..count).filter_map(|i| self.get_sub_type(i)).collect()
    }

    /// Gets all enum values as a vector
    pub fn get_all_enum_values(&self) -> Vec<(String, i32)> {
        let count = self.get_enum_value_count();
        (0..count)
            .filter_map(|i| self.get_enum_value_by_index(i))
            .collect()
    }

    /// Gets all behaviours as a vector
    pub fn get_all_behaviours(&self) -> Vec<(Function, Behaviour)> {
        let count = self.get_behaviour_count();
        (0..count)
            .filter_map(|i| self.get_behaviour_by_index(i))
            .collect()
    }

    /// Gets all child funcdefs as a vector
    pub fn get_all_child_funcdefs(&self) -> Vec<TypeInfo> {
        let count = self.get_child_funcdef_count();
        (0..count)
            .filter_map(|i| self.get_child_funcdef(i))
            .collect()
    }

    /// Finds a method by name
    pub fn find_method(&self, name: &str, get_virtual: bool) -> Option<Function> {
        self.get_method_by_name(name, get_virtual)
    }

    /// Finds a property by name
    pub fn find_property(&self, name: &str) -> Option<(asUINT, TypePropertyInfo)> {
        let count = self.get_property_count();
        for i in 0..count {
            if let Ok(prop) = self.get_property(i) {
                if let Some(prop_name) = &prop.name {
                    if prop_name == name {
                        return Some((i, prop));
                    }
                }
            }
        }
        None
    }

    /// Gets the inheritance chain (this type and all base types)
    pub fn get_inheritance_chain(&self) -> Vec<TypeInfo> {
        let mut chain = vec![self.clone()];
        let mut current = self.clone();

        while let Some(base) = current.get_base_type() {
            chain.push(base.clone());
            current = base;
        }

        chain
    }

    /// Checks if this type can be cast to another type
    pub fn can_cast_to(&self, target_type: &TypeInfo) -> bool {
        // Same type
        if self.get_type_id() == target_type.get_type_id() {
            return true;
        }

        // Check inheritance
        if self.derives_from(target_type) {
            return true;
        }

        // Check interfaces
        if target_type.is_interface() && self.implements(target_type) {
            return true;
        }

        false
    }
}

// ========== ADDITIONAL TYPES ==========

/// Information about a type property
#[derive(Debug, Clone)]
pub struct TypePropertyInfo {
    pub name: Option<String>,
    pub type_id: i32,
    pub is_private: bool,
    pub is_protected: bool,
    pub offset: i32,
    pub is_reference: bool,
    pub access_mask: asDWORD,
    pub composite_offset: i32,
    pub is_composite_indirect: bool,
}

impl TypePropertyInfo {
    /// Checks if the property is public
    pub fn is_public(&self) -> bool {
        !self.is_private && !self.is_protected
    }

    /// Gets the visibility of the property
    pub fn get_visibility(&self) -> PropertyVisibility {
        if self.is_private {
            PropertyVisibility::Private
        } else if self.is_protected {
            PropertyVisibility::Protected
        } else {
            PropertyVisibility::Public
        }
    }
}

/// Property visibility levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyVisibility {
    Public,
    Protected,
    Private,
}

/// Type classification for easier type checking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeKind {
    Class,
    Interface,
    Enum,
    Typedef,
    Funcdef,
    Primitive,
    Unknown,
}

impl TypeInfo {
    /// Gets the kind of this type
    pub fn get_kind(&self) -> TypeKind {
        if self.is_class() {
            TypeKind::Class
        } else if self.is_interface() {
            TypeKind::Interface
        } else if self.is_enum() {
            TypeKind::Enum
        } else if self.is_typedef() {
            TypeKind::Typedef
        } else if self.is_funcdef() {
            TypeKind::Funcdef
        } else {
            TypeKind::Unknown
        }
    }
}
