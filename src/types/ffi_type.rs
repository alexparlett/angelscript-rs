//! Owned type definitions for FFI registry.
//!
//! This module provides `FfiTypeDef`, an owned type definition
//! that can be stored in `Arc<FfiRegistry>` without arena lifetimes.
//!
//! This is the core type for registering native classes with the FFI system.

use crate::ffi::{ListBehavior, NativeFn, TemplateInstanceInfo, TemplateValidation};
use crate::types::{FunctionBuilder, FfiPropertyDef, TypeHash, TypeKind};
use std::sync::Arc;

/// A native type definition.
///
/// This is an owned type definition that can be stored in `Arc<FfiRegistry>`
/// without arena lifetimes.
///
/// # Example
///
/// ```ignore
/// let type_def = FfiTypeDef::new::<MyClass>("MyClass", TypeKind::Reference);
/// ```
pub struct FfiTypeDef {
    /// Unique FFI type ID (assigned at registration via TypeHash::from_name("test_type"))
    pub id: TypeHash,

    /// Type name (unqualified)
    pub name: String,

    /// Template parameters (e.g., ["T"] or ["K", "V"])
    /// Empty if not a template type.
    pub template_params: Vec<String>,

    /// Type kind (value or reference)
    pub type_kind: TypeKind,

    // === Behaviors (map to TypeBehaviors during import) ===
    /// Constructors - initialize value in pre-allocated memory (value types)
    /// Multiple overloads supported. Maps to TypeBehaviors.constructors
    pub constructors: Vec<FunctionBuilder>,

    /// Factory functions - create new instance (reference types)
    /// Multiple overloads supported. Maps to TypeBehaviors.factories
    pub factories: Vec<FunctionBuilder>,

    /// AddRef - increment reference count (reference types)
    pub addref: Option<NativeFn>,

    /// Release - decrement reference count, delete if zero (reference types)
    pub release: Option<NativeFn>,

    /// Destructor - cleanup before deallocation (value types)
    pub destruct: Option<NativeFn>,

    /// List constructor - construct from initialization list (value types)
    pub list_construct: Option<ListBehavior>,

    /// List factory - create from initialization list (reference types)
    pub list_factory: Option<ListBehavior>,

    /// Get weak reference flag - returns a shared weak ref flag object
    pub get_weakref_flag: Option<NativeFn>,

    /// Template callback - validates template instantiation
    /// Uses Arc so it can be shared/cloned during import without ownership transfer
    pub template_callback:
        Option<Arc<dyn Fn(&TemplateInstanceInfo) -> TemplateValidation + Send + Sync>>,

    // === Type members ===
    /// Methods
    pub methods: Vec<FunctionBuilder>,

    /// Properties
    pub properties: Vec<FfiPropertyDef>,

    /// Operators
    pub operators: Vec<FunctionBuilder>,

    /// Rust TypeHash for runtime type checking
    pub rust_type_id: std::any::TypeId,
}

impl FfiTypeDef {
    /// Create a new type definition.
    pub fn new<T: 'static>(id: TypeHash, name: impl Into<String>, type_kind: TypeKind) -> Self {
        Self {
            id,
            name: name.into(),
            template_params: Vec::new(),
            type_kind,
            constructors: Vec::new(),
            factories: Vec::new(),
            addref: None,
            release: None,
            destruct: None,
            list_construct: None,
            list_factory: None,
            get_weakref_flag: None,
            template_callback: None,
            methods: Vec::new(),
            properties: Vec::new(),
            operators: Vec::new(),
            rust_type_id: std::any::TypeId::of::<T>(),
        }
    }

    /// Create a new template type definition.
    pub fn new_template<T: 'static>(
        id: TypeHash,
        name: impl Into<String>,
        template_params: Vec<String>,
        type_kind: TypeKind,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            template_params,
            type_kind,
            constructors: Vec::new(),
            factories: Vec::new(),
            addref: None,
            release: None,
            destruct: None,
            list_construct: None,
            list_factory: None,
            get_weakref_flag: None,
            template_callback: None,
            methods: Vec::new(),
            properties: Vec::new(),
            operators: Vec::new(),
            rust_type_id: std::any::TypeId::of::<T>(),
        }
    }

    /// Get the type name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Check if this is a template type.
    pub fn is_template(&self) -> bool {
        !self.template_params.is_empty()
    }

    /// Check if this is a value type.
    pub fn is_value_type(&self) -> bool {
        matches!(self.type_kind, TypeKind::Value { .. })
    }

    /// Check if this is a reference type.
    pub fn is_reference_type(&self) -> bool {
        matches!(self.type_kind, TypeKind::Reference { .. })
    }

    /// Add a constructor.
    pub fn add_constructor(&mut self, constructor: FunctionBuilder) {
        self.constructors.push(constructor);
    }

    /// Add a factory.
    pub fn add_factory(&mut self, factory: FunctionBuilder) {
        self.factories.push(factory);
    }

    /// Add a method.
    pub fn add_method(&mut self, method: FunctionBuilder) {
        self.methods.push(method);
    }

    /// Add a property.
    pub fn add_property(&mut self, property: FfiPropertyDef) {
        self.properties.push(property);
    }

    /// Add an operator.
    pub fn add_operator(&mut self, operator: FunctionBuilder) {
        self.operators.push(operator);
    }

    /// Set the addref behavior.
    pub fn set_addref(&mut self, addref: NativeFn) {
        self.addref = Some(addref);
    }

    /// Set the release behavior.
    pub fn set_release(&mut self, release: NativeFn) {
        self.release = Some(release);
    }

    /// Set the destructor behavior.
    pub fn set_destruct(&mut self, destruct: NativeFn) {
        self.destruct = Some(destruct);
    }

    /// Set the list constructor behavior.
    pub fn set_list_construct(&mut self, behavior: ListBehavior) {
        self.list_construct = Some(behavior);
    }

    /// Set the list factory behavior.
    pub fn set_list_factory(&mut self, behavior: ListBehavior) {
        self.list_factory = Some(behavior);
    }

    /// Set the get_weakref_flag behavior.
    pub fn set_get_weakref_flag(&mut self, func: NativeFn) {
        self.get_weakref_flag = Some(func);
    }

    /// Set the template callback.
    pub fn set_template_callback<F>(&mut self, callback: F)
    where
        F: Fn(&TemplateInstanceInfo) -> TemplateValidation + Send + Sync + 'static,
    {
        self.template_callback = Some(Arc::new(callback));
    }
}

impl std::fmt::Debug for FfiTypeDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FfiTypeDef")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("template_params", &self.template_params)
            .field("type_kind", &self.type_kind)
            .field("constructors", &self.constructors.len())
            .field("factories", &self.factories.len())
            .field("addref", &self.addref.as_ref().map(|_| "..."))
            .field("release", &self.release.as_ref().map(|_| "..."))
            .field("destruct", &self.destruct.as_ref().map(|_| "..."))
            .field("list_construct", &self.list_construct)
            .field("list_factory", &self.list_factory)
            .field(
                "get_weakref_flag",
                &self.get_weakref_flag.as_ref().map(|_| "..."),
            )
            .field(
                "template_callback",
                &self.template_callback.as_ref().map(|_| "..."),
            )
            .field("methods", &self.methods.len())
            .field("properties", &self.properties.len())
            .field("operators", &self.operators.len())
            .field("rust_type_id", &self.rust_type_id)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestClass;

    #[test]
    fn type_def_creation() {
        let type_def =
            FfiTypeDef::new::<TestClass>(TypeHash::from_name("test_type"), "TestClass", TypeKind::reference());

        assert_eq!(type_def.name(), "TestClass");
        assert!(!type_def.is_template());
        assert!(type_def.is_reference_type());
        assert!(!type_def.is_value_type());
    }

    #[test]
    fn template_type_def_creation() {
        let type_def = FfiTypeDef::new_template::<TestClass>(
            TypeHash::from_name("test_type"),
            "Container",
            vec!["T".to_string()],
            TypeKind::reference(),
        );

        assert_eq!(type_def.name(), "Container");
        assert!(type_def.is_template());
        assert_eq!(type_def.template_params.len(), 1);
        assert_eq!(type_def.template_params[0], "T");
    }

    #[test]
    fn value_type_def() {
        let type_def = FfiTypeDef::new::<TestClass>(
            TypeHash::from_name("test_type"),
            "Vec3",
            TypeKind::Value { size: 12, align: 4, is_pod: false },
        );

        assert!(type_def.is_value_type());
        assert!(!type_def.is_reference_type());
    }

    #[test]
    fn debug_output() {
        let type_def =
            FfiTypeDef::new::<TestClass>(TypeHash::from_name("test_type"), "TestClass", TypeKind::reference());
        let debug = format!("{:?}", type_def);
        assert!(debug.contains("FfiTypeDef"));
        assert!(debug.contains("TestClass"));
    }
}
