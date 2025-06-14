use crate::core::engine::Engine;
use crate::types::enums::*;
use crate::core::error::{ScriptError, ScriptResult};
use crate::core::function::Function;
use crate::core::module::Module;
use crate::types::script_data::ScriptData;
use crate::types::user_data::UserData;
use angelscript_sys::*;
use std::ffi::CStr;
use std::ptr::NonNull;

/// Comprehensive type information for AngelScript types.
///
/// TypeInfo provides detailed metadata about types registered in AngelScript,
/// including classes, interfaces, enums, typedefs, and function definitions.
/// It allows introspection of type properties, methods, inheritance relationships,
/// and other type-specific information.
///
/// # Type Categories
///
/// AngelScript supports several categories of types:
///
/// - **Classes**: Reference or value types with methods and properties
/// - **Interfaces**: Abstract types defining method contracts
/// - **Enums**: Named integer constants
/// - **Typedefs**: Type aliases
/// - **Funcdefs**: Function pointer types
/// - **Primitives**: Built-in types like int, float, string
///
/// # Examples
///
/// ## Basic Type Information
///
/// ```rust
/// use angelscript_rs::{Engine, GetModuleFlags};
///
/// let engine = Engine::create()?;
/// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
///
/// module.add_script_section("types", r#"
///     class Player {
///         string name;
///         int score;
///
///         Player(const string &in n) { name = n; score = 0; }
///         void addScore(int points) { score += points; }
///         int getScore() const { return score; }
///     }
/// "#)?;
/// module.build()?;
///
/// if let Some(player_type) = module.get_type_info_by_name("Player") {
///     println!("Type: {}", player_type.get_full_name());
///     println!("Size: {} bytes", player_type.get_size());
///     println!("Methods: {}", player_type.get_method_count());
///     println!("Properties: {}", player_type.get_property_count());
/// }
/// ```
///
/// ## Inheritance and Interfaces
///
/// ```rust
/// module.add_script_section("inheritance", r#"
///     interface IDrawable {
///         void draw();
///     }
///
///     class Shape : IDrawable {
///         float x, y;
///         void draw() { /* base implementation */ }
///     }
///
///     class Circle : Shape {
///         float radius;
///         void draw() { /* circle-specific drawing */ }
///     }
/// "#)?;
/// module.build()?;
///
/// if let Some(circle_type) = module.get_type_info_by_name("Circle") {
///     // Check inheritance
///     if let Some(shape_type) = module.get_type_info_by_name("Shape") {
///         assert!(circle_type.derives_from(&shape_type));
///     }
///
///     // Check interface implementation
///     if let Some(drawable_type) = module.get_type_info_by_name("IDrawable") {
///         assert!(circle_type.implements(&drawable_type));
///     }
///
///     // Get inheritance chain
///     let chain = circle_type.get_inheritance_chain();
///     for (i, ancestor) in chain.iter().enumerate() {
///         println!("Level {}: {}", i, ancestor.get_full_name());
///     }
/// }
/// ```
///
/// ## Enum Introspection
///
/// ```rust
/// module.add_script_section("enums", r#"
///     enum Color {
///         Red = 1,
///         Green = 2,
///         Blue = 4,
///         Yellow = Red | Green
///     }
/// "#)?;
/// module.build()?;
///
/// if let Some(color_type) = module.get_type_info_by_name("Color") {
///     println!("Enum values:");
///     for (name, value) in color_type.get_all_enum_values() {
///         println!("  {} = {}", name, value);
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct TypeInfo {
    inner: *mut asITypeInfo,
}

impl TypeInfo {
    /// Creates a TypeInfo wrapper from a raw AngelScript pointer.
    ///
    /// # Safety
    /// The pointer must be valid and point to a properly initialized asITypeInfo.
    ///
    /// # Arguments
    /// * `ptr` - Raw pointer to AngelScript type info
    ///
    /// # Returns
    /// A new TypeInfo wrapper
    pub fn from_raw(ptr: *mut asITypeInfo) -> Self {
        let wrapper = Self { inner: ptr };
        wrapper
            .add_ref()
            .expect("Failed to add reference to type info");
        wrapper
    }

    /// Gets the raw AngelScript type info pointer.
    ///
    /// # Safety
    /// This function provides direct access to the AngelScript type info pointer.
    pub(crate) fn as_ptr(&self) -> *mut asITypeInfo {
        self.inner
    }

    /// Checks if the type info pointer is null.
    ///
    /// # Returns
    /// true if the pointer is null, false otherwise
    pub fn is_null(&self) -> bool {
        self.inner.is_null()
    }

    // ========== VTABLE ORDER (matches asITypeInfo__bindgen_vtable) ==========

    /// Gets the engine that owns this type.
    ///
    /// # Returns
    /// The engine instance or an error if not available
    ///
    /// # Examples
    ///
    /// ```rust
    /// let type_info = module.get_type_info_by_name("MyClass")?;
    /// let engine = type_info.get_engine()?;
    /// ```
    pub fn get_engine(&self) -> ScriptResult<Engine> {
        unsafe {
            let result: *mut asIScriptEngine =
                (self.as_vtable().asITypeInfo_GetEngine)(self.inner);
            let ptr = NonNull::new(result).ok_or(ScriptError::NullPointer)?;
            Ok(Engine::from_raw(ptr))
        }
    }

    /// Gets the configuration group this type belongs to.
    ///
    /// Configuration groups allow batch removal of related registrations.
    ///
    /// # Returns
    /// The configuration group name, or None if not in a group
    ///
    /// # Examples
    ///
    /// ```rust
    /// let type_info = engine.get_type_info_by_name("MyRegisteredType")?;
    /// if let Some(group) = type_info.get_config_group() {
    ///     println!("Type is in config group: {}", group);
    /// }
    /// ```
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

    /// Gets the access mask for this type.
    ///
    /// Access masks control which modules can access this type.
    ///
    /// # Returns
    /// The access mask as a bitmask
    ///
    /// # Examples
    ///
    /// ```rust
    /// let type_info = module.get_type_info_by_name("MyClass")?;
    /// let access_mask = type_info.get_access_mask();
    /// if access_mask & 0x01 != 0 {
    ///     println!("Type is accessible to group 1");
    /// }
    /// ```
    pub fn get_access_mask(&self) -> asDWORD {
        unsafe { (self.as_vtable().asITypeInfo_GetAccessMask)(self.inner) }
    }

    /// Gets the module that contains this type.
    ///
    /// # Returns
    /// The module instance, or None if the type is not part of a module
    ///
    /// # Examples
    ///
    /// ```rust
    /// let type_info = module.get_type_info_by_name("MyClass")?;
    /// if let Some(type_module) = type_info.get_module() {
    ///     println!("Type belongs to module: {:?}", type_module.get_name());
    /// }
    /// ```
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

    /// Increments the reference count of the type info.
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn add_ref(&self) -> ScriptResult<()> {
        unsafe { ScriptError::from_code((self.as_vtable().asITypeInfo_AddRef)(self.inner)) }
    }

    /// Decrements the reference count of the type info.
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn release(&self) -> ScriptResult<()> {
        unsafe { ScriptError::from_code((self.as_vtable().asITypeInfo_Release)(self.inner)) }
    }

    /// Gets the name of this type.
    ///
    /// # Returns
    /// The type name, or None if not available
    ///
    /// # Examples
    ///
    /// ```rust
    /// let type_info = module.get_type_info_by_name("MyClass")?;
    /// assert_eq!(type_info.get_name(), Some("MyClass"));
    /// ```
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

    /// Gets the namespace this type belongs to.
    ///
    /// # Returns
    /// The namespace name, or None if in the global namespace
    ///
    /// # Examples
    ///
    /// ```rust
    /// // For a type declared as "namespace Math { class Vector3 {} }"
    /// let type_info = module.get_type_info_by_name("Vector3")?;
    /// assert_eq!(type_info.get_namespace(), Some("Math"));
    /// ```
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

    /// Gets the base type this type inherits from.
    ///
    /// # Returns
    /// The base type, or None if this type doesn't inherit from another type
    ///
    /// # Examples
    ///
    /// ```rust
    /// // For "class Derived : Base {}"
    /// let derived_type = module.get_type_info_by_name("Derived")?;
    /// if let Some(base_type) = derived_type.get_base_type() {
    ///     println!("Derived inherits from: {}", base_type.get_name().unwrap_or("unknown"));
    /// }
    /// ```
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

    /// Checks if this type derives from another type.
    ///
    /// This checks the entire inheritance chain, not just direct inheritance.
    ///
    /// # Arguments
    /// * `obj_type` - The potential base type to check against
    ///
    /// # Returns
    /// true if this type derives from the given type, false otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// // For "class Child : Parent : Grandparent {}"
    /// let child_type = module.get_type_info_by_name("Child")?;
    /// let grandparent_type = module.get_type_info_by_name("Grandparent")?;
    ///
    /// assert!(child_type.derives_from(&grandparent_type));
    /// ```
    pub fn derives_from(&self, obj_type: &TypeInfo) -> bool {
        unsafe { (self.as_vtable().asITypeInfo_DerivesFrom)(self.inner, obj_type.inner) }
    }

    /// Gets the flags describing this type's characteristics.
    ///
    /// # Returns
    /// Object type flags indicating properties like reference type, value type, etc.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let type_info = module.get_type_info_by_name("MyClass")?;
    /// let flags = type_info.get_flags();
    ///
    /// if flags.contains(ObjectTypeFlags::REF) {
    ///     println!("This is a reference type");
    /// }
    /// if flags.contains(ObjectTypeFlags::VALUE) {
    ///     println!("This is a value type");
    /// }
    /// ```
    pub fn get_flags(&self) -> ObjectTypeFlags {
        unsafe { ((self.as_vtable().asITypeInfo_GetFlags)(self.inner)).into() }
    }

    /// Gets the size of this type in bytes.
    ///
    /// # Returns
    /// The size in bytes, or 0 for interfaces and some special types
    ///
    /// # Examples
    ///
    /// ```rust
    /// let type_info = module.get_type_info_by_name("MyClass")?;
    /// println!("MyClass size: {} bytes", type_info.get_size());
    /// ```
    pub fn get_size(&self) -> asUINT {
        unsafe { (self.as_vtable().asITypeInfo_GetSize)(self.inner) }
    }

    /// Gets the type ID for this type.
    ///
    /// Type IDs are unique identifiers used throughout AngelScript for type identification.
    ///
    /// # Returns
    /// The type ID
    ///
    /// # Examples
    ///
    /// ```rust
    /// let type_info = module.get_type_info_by_name("MyClass")?;
    /// let type_id = type_info.get_type_id();
    ///
    /// // Use type_id for variable declarations, function parameters, etc.
    /// ```
    pub fn get_type_id(&self) -> TypeId {
        unsafe { TypeId::from((self.as_vtable().asITypeInfo_GetTypeId)(self.inner) as asUINT) }
    }

    /// Gets the type ID of a sub-type.
    ///
    /// Sub-types are used for template types like arrays or other generic containers.
    ///
    /// # Arguments
    /// * `sub_type_index` - Index of the sub-type (0-based)
    ///
    /// # Returns
    /// The sub-type's type ID
    ///
    /// # Examples
    ///
    /// ```rust
    /// // For array<int>, the sub-type would be int
    /// let array_type = engine.get_type_info_by_decl("array<int>")?;
    /// if array_type.get_sub_type_count() > 0 {
    ///     let element_type_id = array_type.get_sub_type_id(0);
    ///     println!("Array element type ID: {}", element_type_id);
    /// }
    /// ```
    pub fn get_sub_type_id(&self, sub_type_index: asUINT) -> TypeId {
        unsafe { TypeId::from((self.as_vtable().asITypeInfo_GetSubTypeId)(self.inner, sub_type_index) as asUINT) }
    }

    /// Gets a sub-type by index.
    ///
    /// # Arguments
    /// * `sub_type_index` - Index of the sub-type (0-based)
    ///
    /// # Returns
    /// The sub-type information, or None if the index is invalid
    ///
    /// # Examples
    ///
    /// ```rust
    /// let array_type = engine.get_type_info_by_decl("array<string>")?;
    /// if let Some(element_type) = array_type.get_sub_type(0) {
    ///     println!("Array contains: {}", element_type.get_name().unwrap_or("unknown"));
    /// }
    /// ```
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

    /// Gets the number of sub-types.
    ///
    /// # Returns
    /// The number of sub-types (0 for non-template types)
    ///
    /// # Examples
    ///
    /// ```rust
    /// let array_type = engine.get_type_info_by_decl("array<int>")?;
    /// assert_eq!(array_type.get_sub_type_count(), 1); // One element type
    ///
    /// let map_type = engine.get_type_info_by_decl("dictionary")?;
    /// // Dictionary might have 2 sub-types: key and value
    /// ```
    pub fn get_sub_type_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asITypeInfo_GetSubTypeCount)(self.inner) }
    }

    /// Gets the number of interfaces this type implements.
    ///
    /// # Returns
    /// The number of implemented interfaces
    ///
    /// # Examples
    ///
    /// ```rust
    /// // For "class MyClass : IDrawable, IUpdatable {}"
    /// let class_type = module.get_type_info_by_name("MyClass")?;
    /// assert_eq!(class_type.get_interface_count(), 2);
    /// ```
    pub fn get_interface_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asITypeInfo_GetInterfaceCount)(self.inner) }
    }

    /// Gets an interface by index.
    ///
    /// # Arguments
    /// * `index` - The interface index (0-based)
    ///
    /// # Returns
    /// The interface type information, or None if the index is invalid
    ///
    /// # Examples
    ///
    /// ```rust
    /// let class_type = module.get_type_info_by_name("MyClass")?;
    /// for i in 0..class_type.get_interface_count() {
    ///     if let Some(interface) = class_type.get_interface(i) {
    ///         println!("Implements interface: {}", interface.get_name().unwrap_or("unknown"));
    ///     }
    /// }
    /// ```
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

    /// Checks if this type implements a specific interface.
    ///
    /// # Arguments
    /// * `obj_type` - The interface type to check
    ///
    /// # Returns
    /// true if this type implements the interface, false otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// let class_type = module.get_type_info_by_name("MyClass")?;
    /// let drawable_interface = module.get_type_info_by_name("IDrawable")?;
    ///
    /// if class_type.implements(&drawable_interface) {
    ///     println!("MyClass can be drawn");
    /// }
    /// ```
    pub fn implements(&self, obj_type: &TypeInfo) -> bool {
        unsafe { (self.as_vtable().asITypeInfo_Implements)(self.inner, obj_type.inner) }
    }

    /// Gets the number of factory functions for this type.
    ///
    /// Factory functions are constructors and other creation methods.
    ///
    /// # Returns
    /// The number of factory functions
    ///
    /// # Examples
    ///
    /// ```rust
    /// let class_type = module.get_type_info_by_name("MyClass")?;
    /// println!("MyClass has {} constructors", class_type.get_factory_count());
    /// ```
    pub fn get_factory_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asITypeInfo_GetFactoryCount)(self.inner) }
    }

    /// Gets a factory function by index.
    ///
    /// # Arguments
    /// * `index` - The factory index (0-based)
    ///
    /// # Returns
    /// The factory function, or None if the index is invalid
    ///
    /// # Examples
    ///
    /// ```rust
    /// let class_type = module.get_type_info_by_name("MyClass")?;
    /// for i in 0..class_type.get_factory_count() {
    ///     if let Some(factory) = class_type.get_factory_by_index(i) {
    ///         println!("Constructor: {}", factory.get_declaration(false, false, true)?);
    ///     }
    /// }
    /// ```
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

    /// Gets a factory function by declaration.
    ///
    /// # Arguments
    /// * `decl` - The factory declaration (e.g., "MyClass(int, string)")
    ///
    /// # Returns
    /// The factory function, or None if not found
    ///
    /// # Examples
    ///
    /// ```rust
    /// let class_type = module.get_type_info_by_name("MyClass")?;
    /// if let Some(constructor) = class_type.get_factory_by_decl("MyClass(int)") {
    ///     println!("Found constructor taking int parameter");
    /// }
    /// ```
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

    /// Gets the number of methods in this type.
    ///
    /// # Returns
    /// The number of methods
    ///
    /// # Examples
    ///
    /// ```rust
    /// let class_type = module.get_type_info_by_name("MyClass")?;
    /// println!("MyClass has {} methods", class_type.get_method_count());
    /// ```
    pub fn get_method_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asITypeInfo_GetMethodCount)(self.inner) }
    }

    /// Gets a method by index.
    ///
    /// # Arguments
    /// * `index` - The method index (0-based)
    /// * `get_virtual` - Whether to get the virtual method implementation
    ///
    /// # Returns
    /// The method function, or None if the index is invalid
    ///
    /// # Examples
    ///
    /// ```rust
    /// let class_type = module.get_type_info_by_name("MyClass")?;
    /// for i in 0..class_type.get_method_count() {
    ///     if let Some(method) = class_type.get_method_by_index(i, false) {
    ///         println!("Method: {}", method.get_name().unwrap_or("unnamed"));
    ///     }
    /// }
    /// ```
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

    /// Gets a method by name.
    ///
    /// If multiple methods have the same name (overloads), this returns the first one found.
    ///
    /// # Arguments
    /// * `name` - The method name
    /// * `get_virtual` - Whether to get the virtual method implementation
    ///
    /// # Returns
    /// The method function, or None if not found
    ///
    /// # Examples
    ///
    /// ```rust
    /// let class_type = module.get_type_info_by_name("MyClass")?;
    /// if let Some(method) = class_type.get_method_by_name("getValue", false) {
    ///     println!("Found getValue method");
    /// }
    /// ```
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

    /// Gets a method by declaration.
    ///
    /// # Arguments
    /// * `decl` - The method declaration (e.g., "void setValue(int)")
    /// * `get_virtual` - Whether to get the virtual method implementation
    ///
    /// # Returns
    /// The method function, or None if not found
    ///
    /// # Examples
    ///
    /// ```rust
    /// let class_type = module.get_type_info_by_name("MyClass")?;
    /// if let Some(method) = class_type.get_method_by_decl("void setValue(int)", false) {
    ///     println!("Found setValue method with int parameter");
    /// }
    /// ```
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

    /// Gets the number of properties in this type.
    ///
    /// # Returns
    /// The number of properties
    ///
    /// # Examples
    ///
    /// ```rust
    /// let class_type = module.get_type_info_by_name("MyClass")?;
    /// println!("MyClass has {} properties", class_type.get_property_count());
    /// ```
    pub fn get_property_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asITypeInfo_GetPropertyCount)(self.inner) }
    }

    /// Gets detailed information about a property.
    ///
    /// # Arguments
    /// * `index` - The property index (0-based)
    ///
    /// # Returns
    /// Property information or an error if the index is invalid
    ///
    /// # Examples
    ///
    /// ```rust
    /// let class_type = module.get_type_info_by_name("MyClass")?;
    /// for i in 0..class_type.get_property_count() {
    ///     match class_type.get_property(i) {
    ///         Ok(prop) => {
    ///             println!("Property: {:?} (type_id: {}, offset: {})",
    ///                      prop.name, prop.type_id, prop.offset);
    ///             println!("  Visibility: {:?}", prop.get_visibility());
    ///         }
    ///         Err(e) => eprintln!("Error getting property {}: {}", i, e),
    ///     }
    /// }
    /// ```
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

    /// Gets the declaration string for a property.
    ///
    /// # Arguments
    /// * `index` - The property index (0-based)
    /// * `include_namespace` - Whether to include namespace in type names
    ///
    /// # Returns
    /// The property declaration, or None if the index is invalid
    ///
    /// # Examples
    ///
    /// ```rust
    /// let class_type = module.get_type_info_by_name("MyClass")?;
    /// for i in 0..class_type.get_property_count() {
    ///     if let Some(decl) = class_type.get_property_declaration(i, true) {
    ///         println!("Property {}: {}", i, decl);
    ///     }
    /// }
    /// ```
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

    /// Gets the number of behaviours (special methods) in this type.
    ///
    /// Behaviours include constructors, destructors, operators, etc.
    ///
    /// # Returns
    /// The number of behaviours
    ///
    /// # Examples
    ///
    /// ```rust
    /// let class_type = module.get_type_info_by_name("MyClass")?;
    /// println!("MyClass has {} behaviours", class_type.get_behaviour_count());
    /// ```
    pub fn get_behaviour_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asITypeInfo_GetBehaviourCount)(self.inner) }
    }

    /// Gets a behaviour by index.
    ///
    /// # Arguments
    /// * `index` - The behaviour index (0-based)
    ///
    /// # Returns
    /// A tuple of (function, behaviour_type), or None if the index is invalid
    ///
    /// # Examples
    ///
    /// ```rust
    /// let class_type = module.get_type_info_by_name("MyClass")?;
    /// for i in 0..class_type.get_behaviour_count() {
    ///     if let Some((func, behaviour)) = class_type.get_behaviour_by_index(i) {
    ///         println!("Behaviour: {:?} - {}", behaviour,
    ///                  func.get_declaration(false, false, true)?);
    ///     }
    /// }
    /// ```
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

    /// Gets the number of child function definitions.
    ///
    /// Child funcdefs are function pointer types defined within this type.
    ///
    /// # Returns
    /// The number of child funcdefs
    ///
    /// # Examples
    ///
    /// ```rust
    /// let class_type = module.get_type_info_by_name("MyClass")?;
    /// println!("MyClass has {} child funcdefs", class_type.get_child_funcdef_count());
    /// ```
    pub fn get_child_funcdef_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asITypeInfo_GetChildFuncdefCount)(self.inner) }
    }

    /// Gets a child function definition by index.
    ///
    /// # Arguments
    /// * `index` - The funcdef index (0-based)
    ///
    /// # Returns
    /// The funcdef type information, or None if the index is invalid
    ///
    /// # Examples
    ///
    /// ```rust
    /// let class_type = module.get_type_info_by_name("MyClass")?;
    /// for i in 0..class_type.get_child_funcdef_count() {
    ///     if let Some(funcdef) = class_type.get_child_funcdef(i) {
    ///         println!("Child funcdef: {}", funcdef.get_name().unwrap_or("unnamed"));
    ///     }
    /// }
    /// ```
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

    /// Gets the parent type for nested types.
    ///
    /// # Returns
    /// The parent type, or None if this is not a nested type
    ///
    /// # Examples
    ///
    /// ```rust
    /// // For nested types like "class Outer { class Inner {} }"
    /// let inner_type = module.get_type_info_by_name("Inner")?;
    /// if let Some(parent) = inner_type.get_parent_type() {
    ///     println!("Inner is nested in: {}", parent.get_name().unwrap_or("unknown"));
    /// }
    /// ```
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

    /// Gets the number of values in an enum.
    ///
    /// # Returns
    /// The number of enum values (0 for non-enum types)
    ///
    /// # Examples
    ///
    /// ```rust
    /// let enum_type = module.get_type_info_by_name("Color")?;
    /// println!("Color enum has {} values", enum_type.get_enum_value_count());
    /// ```
    pub fn get_enum_value_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asITypeInfo_GetEnumValueCount)(self.inner) }
    }

    /// Gets an enum value by index.
    ///
    /// # Arguments
    /// * `index` - The enum value index (0-based)
    ///
    /// # Returns
    /// A tuple of (name, value), or None if the index is invalid
    ///
    /// # Examples
    ///
    /// ```rust
    /// let enum_type = module.get_type_info_by_name("Color")?;
    /// for i in 0..enum_type.get_enum_value_count() {
    ///     if let Some((name, value)) = enum_type.get_enum_value_by_index(i) {
    ///         println!("{} = {}", name, value);
    ///     }
    /// }
    /// ```
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

    /// Gets the underlying type ID for a typedef.
    ///
    /// # Returns
    /// The type ID of the aliased type (for typedef types only)
    ///
    /// # Examples
    ///
    /// ```rust
    /// // For "typedef int MyInt;"
    /// let typedef_type = module.get_type_info_by_name("MyInt")?;
    /// let underlying_type_id = typedef_type.get_typedef_type_id();
    /// // underlying_type_id would be the type ID for int
    /// ```
    pub fn get_typedef_type_id(&self) -> i32 {
        unsafe { (self.as_vtable().asITypeInfo_GetTypedefTypeId)(self.inner) }
    }

    /// Gets the function signature for a funcdef.
    ///
    /// # Returns
    /// The function signature, or None if this is not a funcdef
    ///
    /// # Examples
    ///
    /// ```rust
    /// // For "funcdef void Callback(int);"
    /// let funcdef_type = module.get_type_info_by_name("Callback")?;
    /// if let Some(signature) = funcdef_type.get_funcdef_signature() {
    ///     println!("Funcdef signature: {}", signature.get_declaration(false, false, true)?);
    /// }
    /// ```
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

    /// Sets user data on this type.
    ///
    /// User data allows applications to associate custom data with types.
    ///
    /// # Arguments
    /// * `data` - The user data to set
    ///
    /// # Returns
    /// The previous user data, or None if none was set
    ///
    /// # Examples
    ///
    /// ```rust
    /// let type_info = module.get_type_info_by_name("MyClass")?;
    /// let mut my_data = MyUserData::new();
    ///
    /// if let Some(old_data) = type_info.set_user_data(&mut my_data) {
    ///     println!("Replaced existing user data");
    /// }
    /// ```
    pub fn set_user_data<T: UserData + ScriptData>(&self, data: &mut T) -> Option<T> {
        unsafe {
            let ptr = (self.as_vtable().asITypeInfo_SetUserData)(
                self.inner,
                data.to_script_ptr(),
                T::KEY as asPWORD,
            );
            if ptr.is_null() {
                None
            } else {
                Some(ScriptData::from_script_ptr(ptr))
            }
        }
    }

    /// Gets user data from this type.
    ///
    /// # Returns
    /// The user data, or None if not found
    ///
    /// # Examples
    ///
    /// ```rust
    /// let type_info = module.get_type_info_by_name("MyClass")?;
    ///
    /// if let Some(data) = type_info.get_user_data::<MyUserData>() {
    ///     println!("Found user data: {:?}", data);
    /// }
    /// ```
    pub fn get_user_data<T: UserData + ScriptData>(&self) -> Option<T> {
        unsafe {
            let ptr = (self.as_vtable().asITypeInfo_GetUserData)(self.inner, T::KEY as asPWORD);
            if ptr.is_null() {
                None
            } else {
                Some(ScriptData::from_script_ptr(ptr))
            }
        }
    }

    /// Gets the vtable for the underlying AngelScript type info.
    ///
    /// # Safety
    /// This function provides direct access to the AngelScript vtable.
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
    /// Gets the full name including namespace.
    ///
    /// # Returns
    /// The full type name with namespace prefix
    ///
    /// # Examples
    ///
    /// ```rust
    /// // For "namespace Math { class Vector3 {} }"
    /// let type_info = module.get_type_info_by_name("Vector3")?;
    /// assert_eq!(type_info.get_full_name(), "Math::Vector3");
    /// ```
    pub fn get_full_name(&self) -> String {
        match (self.get_namespace(), self.get_name()) {
            (Some(ns), Some(name)) if !ns.is_empty() => format!("{}::{}", ns, name),
            (_, Some(name)) => name.to_string(),
            _ => "<unknown>".to_string(),
        }
    }

    /// Checks if this type is a specific type by name.
    ///
    /// # Arguments
    /// * `type_name` - The type name to check against
    ///
    /// # Returns
    /// true if the type name matches, false otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// let type_info = module.get_type_info_by_name("MyClass")?;
    /// assert!(type_info.is_type("MyClass"));
    /// assert!(!type_info.is_type("OtherClass"));
    /// ```
    pub fn is_type(&self, type_name: &str) -> bool {
        self.get_name() == Some(type_name)
    }

    /// Checks if this type is a class.
    ///
    /// # Returns
    /// true if this is a class type, false otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// let class_type = module.get_type_info_by_name("MyClass")?;
    /// assert!(class_type.is_class());
    ///
    /// let interface_type = module.get_type_info_by_name("IMyInterface")?;
    /// assert!(!interface_type.is_class());
    /// ```
    pub fn is_class(&self) -> bool {
        let flags = self.get_flags();
        (flags & ObjectTypeFlags::REF).is_empty() || (flags & ObjectTypeFlags::VALUE).is_empty()
    }

    /// Checks if this type is an interface.
    ///
    /// # Returns
    /// true if this is an interface type, false otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// let interface_type = module.get_type_info_by_name("IDrawable")?;
    /// assert!(interface_type.is_interface());
    /// ```
    pub fn is_interface(&self) -> bool {
        // Interfaces typically have specific flags or can be determined by having no size
        self.get_size() == 0 && self.get_method_count() > 0
    }

    /// Checks if this type is an enum.
    ///
    /// # Returns
    /// true if this is an enum type, false otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// let enum_type = module.get_type_info_by_name("Color")?;
    /// assert!(enum_type.is_enum());
    /// ```
    pub fn is_enum(&self) -> bool {
        let flags = self.get_flags();
        (flags & ObjectTypeFlags::ENUM).is_empty()
    }

    /// Checks if this type is a typedef.
    ///
    /// # Returns
    /// true if this is a typedef, false otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// let typedef_type = module.get_type_info_by_name("MyInt")?;
    /// assert!(typedef_type.is_typedef());
    /// ```
    pub fn is_typedef(&self) -> bool {
        let flags = self.get_flags();
        (flags & ObjectTypeFlags::TYPEDEF).is_empty()
    }

    /// Checks if this type is a function definition.
    ///
    /// # Returns
    /// true if this is a funcdef, false otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// let funcdef_type = module.get_type_info_by_name("Callback")?;
    /// assert!(funcdef_type.is_funcdef());
    /// ```
    pub fn is_funcdef(&self) -> bool {
        let flags = self.get_flags();
        (flags & ObjectTypeFlags::FUNCDEF).is_empty()
    }

    /// Gets all methods as a vector.
    ///
    /// # Arguments
    /// * `get_virtual` - Whether to get virtual method implementations
    ///
    /// # Returns
    /// A vector containing all methods
    ///
    /// # Examples
    ///
    /// ```rust
    /// let class_type = module.get_type_info_by_name("MyClass")?;
    /// let methods = class_type.get_all_methods(false);
    ///
    /// for method in methods {
    ///     println!("Method: {}", method.get_name().unwrap_or("unnamed"));
    /// }
    /// ```
    pub fn get_all_methods(&self, get_virtual: bool) -> Vec<Function> {
        let count = self.get_method_count();
        (0..count)
            .filter_map(|i| self.get_method_by_index(i, get_virtual))
            .collect()
    }

    /// Gets all factories as a vector.
    ///
    /// # Returns
    /// A vector containing all factory functions
    ///
    /// # Examples
    ///
    /// ```rust
    /// let class_type = module.get_type_info_by_name("MyClass")?;
    /// let factories = class_type.get_all_factories();
    ///
    /// for factory in factories {
    ///     println!("Constructor: {}", factory.get_declaration(false, false, true)?);
    /// }
    /// ```
    pub fn get_all_factories(&self) -> Vec<Function> {
        let count = self.get_factory_count();
        (0..count)
            .filter_map(|i| self.get_factory_by_index(i))
            .collect()
    }

    /// Gets all properties as a vector.
    ///
    /// # Returns
    /// A vector containing all property information
    ///
    /// # Examples
    ///
    /// ```rust
    /// let class_type = module.get_type_info_by_name("MyClass")?;
    /// let properties = class_type.get_all_properties();
    ///
    /// for prop in properties {
    ///     println!("Property: {:?} (visibility: {:?})", prop.name, prop.get_visibility());
    /// }
    /// ```
    pub fn get_all_properties(&self) -> Vec<TypePropertyInfo> {
        let count = self.get_property_count();
        (0..count)
            .filter_map(|i| self.get_property(i).ok())
            .collect()
    }

    /// Gets all interfaces as a vector.
    ///
    /// # Returns
    /// A vector containing all implemented interfaces
    ///
    /// # Examples
    ///
    /// ```rust
    /// let class_type = module.get_type_info_by_name("MyClass")?;
    /// let interfaces = class_type.get_all_interfaces();
    ///
    /// for interface in interfaces {
    ///     println!("Implements: {}", interface.get_name().unwrap_or("unnamed"));
    /// }
    /// ```
    pub fn get_all_interfaces(&self) -> Vec<TypeInfo> {
        let count = self.get_interface_count();
        (0..count).filter_map(|i| self.get_interface(i)).collect()
    }

    /// Gets all sub-types as a vector.
    ///
    /// # Returns
    /// A vector containing all sub-types
    ///
    /// # Examples
    ///
    /// ```rust
    /// let array_type = engine.get_type_info_by_decl("array<string>")?;
    /// let sub_types = array_type.get_all_sub_types();
    ///
    /// for sub_type in sub_types {
    ///     println!("Element type: {}", sub_type.get_name().unwrap_or("unnamed"));
    /// }
    /// ```
    pub fn get_all_sub_types(&self) -> Vec<TypeInfo> {
        let count = self.get_sub_type_count();
        (0..count).filter_map(|i| self.get_sub_type(i)).collect()
    }

    /// Gets all enum values as a vector.
    ///
    /// # Returns
    /// A vector containing all enum name-value pairs
    ///
    /// # Examples
    ///
    /// ```rust
    /// let enum_type = module.get_type_info_by_name("Color")?;
    /// let values = enum_type.get_all_enum_values();
    ///
    /// for (name, value) in values {
    ///     println!("{} = {}", name, value);
    /// }
    /// ```
    pub fn get_all_enum_values(&self) -> Vec<(String, i32)> {
        let count = self.get_enum_value_count();
        (0..count)
            .filter_map(|i| self.get_enum_value_by_index(i))
            .collect()
    }

    /// Gets all behaviours as a vector.
    ///
    /// # Returns
    /// A vector containing all behaviour function-type pairs
    ///
    /// # Examples
    ///
    /// ```rust
    /// let class_type = module.get_type_info_by_name("MyClass")?;
    /// let behaviours = class_type.get_all_behaviours();
    ///
    /// for (func, behaviour) in behaviours {
    ///     println!("Behaviour {:?}: {}", behaviour,
    ///              func.get_declaration(false, false, true)?);
    /// }
    /// ```
    pub fn get_all_behaviours(&self) -> Vec<(Function, Behaviour)> {
        let count = self.get_behaviour_count();
        (0..count)
            .filter_map(|i| self.get_behaviour_by_index(i))
            .collect()
    }

    /// Gets all child funcdefs as a vector.
    ///
    /// # Returns
    /// A vector containing all child function definitions
    ///
    /// # Examples
    ///
    /// ```rust
    /// let class_type = module.get_type_info_by_name("MyClass")?;
    /// let funcdefs = class_type.get_all_child_funcdefs();
    ///
    /// for funcdef in funcdefs {
    ///     println!("Child funcdef: {}", funcdef.get_name().unwrap_or("unnamed"));
    /// }
    /// ```
    pub fn get_all_child_funcdefs(&self) -> Vec<TypeInfo> {
        let count = self.get_child_funcdef_count();
        (0..count)
            .filter_map(|i| self.get_child_funcdef(i))
            .collect()
    }

    /// Finds a method by name.
    ///
    /// This is an alias for `get_method_by_name` for convenience.
    ///
    /// # Arguments
    /// * `name` - The method name
    /// * `get_virtual` - Whether to get virtual method implementations
    ///
    /// # Returns
    /// The method function, or None if not found
    ///
    /// # Examples
    ///
    /// ```rust
    /// let class_type = module.get_type_info_by_name("MyClass")?;
    /// if let Some(method) = class_type.find_method("getValue", false) {
    ///     println!("Found getValue method");
    /// }
    /// ```
    pub fn find_method(&self, name: &str, get_virtual: bool) -> Option<Function> {
        self.get_method_by_name(name, get_virtual)
    }

    /// Finds a property by name.
    ///
    /// # Arguments
    /// * `name` - The property name
    ///
    /// # Returns
    /// A tuple of (index, property_info), or None if not found
    ///
    /// # Examples
    ///
    /// ```rust
    /// let class_type = module.get_type_info_by_name("MyClass")?;
    /// if let Some((index, prop)) = class_type.find_property("value") {
    ///     println!("Property 'value' found at index {} with type_id {}", index, prop.type_id);
    /// }
    /// ```
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

    /// Gets the inheritance chain (this type and all base types).
    ///
    /// # Returns
    /// A vector containing this type and all its ancestors
    ///
    /// # Examples
    ///
    /// ```rust
    /// // For "class Child : Parent : Grandparent {}"
    /// let child_type = module.get_type_info_by_name("Child")?;
    /// let chain = child_type.get_inheritance_chain();
    ///
    /// // chain[0] = Child, chain[1] = Parent, chain[2] = Grandparent
    /// for (level, ancestor) in chain.iter().enumerate() {
    ///     println!("Level {}: {}", level, ancestor.get_name().unwrap_or("unknown"));
    /// }
    /// ```
    pub fn get_inheritance_chain(&self) -> Vec<TypeInfo> {
        let mut chain = vec![self.clone()];
        let mut current = self.clone();

        while let Some(base) = current.get_base_type() {
            chain.push(base.clone());
            current = base;
        }

        chain
    }

    /// Checks if this type can be cast to another type.
    ///
    /// This checks for valid casting relationships including inheritance and interface implementation.
    ///
    /// # Arguments
    /// * `target_type` - The target type to check casting to
    ///
    /// # Returns
    /// true if casting is possible, false otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// let child_type = module.get_type_info_by_name("Child")?;
    /// let parent_type = module.get_type_info_by_name("Parent")?;
    /// let interface_type = module.get_type_info_by_name("IDrawable")?;
    ///
    /// assert!(child_type.can_cast_to(&parent_type)); // Inheritance
    /// assert!(child_type.can_cast_to(&interface_type)); // Interface implementation
    /// assert!(child_type.can_cast_to(&child_type)); // Same type
    /// ```
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

/// Information about a type property.
///
/// This struct contains detailed information about a class property,
/// including its type, visibility, memory layout, and access characteristics.
#[derive(Debug, Clone)]
pub struct TypePropertyInfo {
    /// The property name
    pub name: Option<String>,
    /// The type ID of the property
    pub type_id: i32,
    /// Whether the property is private
    pub is_private: bool,
    /// Whether the property is protected
    pub is_protected: bool,
    /// Byte offset of the property in the object
    pub offset: i32,
    /// Whether the property is a reference
    pub is_reference: bool,
    /// Access mask for the property
    pub access_mask: asDWORD,
    /// Composite offset for complex types
    pub composite_offset: i32,
    /// Whether the composite is indirect
    pub is_composite_indirect: bool,
}

impl TypePropertyInfo {
    /// Checks if the property is public.
    ///
    /// # Returns
    /// true if the property is public (not private or protected)
    ///
    /// # Examples
    ///
    /// ```rust
    /// let prop_info = class_type.get_property(0)?;
    /// if prop_info.is_public() {
    ///     println!("Property is publicly accessible");
    /// }
    /// ```
    pub fn is_public(&self) -> bool {
        !self.is_private && !self.is_protected
    }

    /// Gets the visibility of the property.
    ///
    /// # Returns
    /// The property visibility level
    ///
    /// # Examples
    ///
    /// ```rust
    /// let prop_info = class_type.get_property(0)?;
    /// match prop_info.get_visibility() {
    ///     PropertyVisibility::Public => println!("Public property"),
    ///     PropertyVisibility::Protected => println!("Protected property"),
    ///     PropertyVisibility::Private => println!("Private property"),
    /// }
    /// ```
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

/// Property visibility levels.
///
/// This enum represents the different access levels for class properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyVisibility {
    /// Property is accessible from anywhere
    Public,
    /// Property is accessible from the class and its subclasses
    Protected,
    /// Property is only accessible from within the same class
    Private,
}

/// Type classification for easier type checking.
///
/// This enum provides a simplified way to categorize types without
/// needing to check multiple flags or conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeKind {
    /// A class type (reference or value type with methods and properties)
    Class,
    /// An interface type (abstract type defining method contracts)
    Interface,
    /// An enum type (named integer constants)
    Enum,
    /// A typedef (type alias)
    Typedef,
    /// A function definition (function pointer type)
    Funcdef,
    /// A primitive type (built-in types like int, float, string)
    Primitive,
    /// Unknown or unclassified type
    Unknown,
}

impl TypeInfo {
    /// Gets the kind of this type.
    ///
    /// This provides a simplified classification of the type for easier handling.
    ///
    /// # Returns
    /// The type kind
    ///
    /// # Examples
    ///
    /// ```rust
    /// let type_info = module.get_type_info_by_name("MyClass")?;
    /// match type_info.get_kind() {
    ///     TypeKind::Class => println!("This is a class"),
    ///     TypeKind::Interface => println!("This is an interface"),
    ///     TypeKind::Enum => println!("This is an enum"),
    ///     TypeKind::Typedef => println!("This is a typedef"),
    ///     TypeKind::Funcdef => println!("This is a funcdef"),
    ///     TypeKind::Primitive => println!("This is a primitive type"),
    ///     TypeKind::Unknown => println!("Unknown type"),
    /// }
    /// ```
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
