//! InterfaceBuilder for registering native interface types with the FFI system.
//!
//! InterfaceBuilder provides a fluent API for registering native interface types
//! with abstract method signatures that scripts can implement.
//!
//! # Example
//!
//! ```ignore
//! // Simple interface
//! module.register_interface("IDrawable")
//!     .method("void draw() const")?
//!     .method("void setVisible(bool visible)")?
//!     .build()?;
//!
//! // Serialization interface
//! module.register_interface("ISerializable")
//!     .method("string serialize() const")?
//!     .method("void deserialize(const string &in data)")?
//!     .build()?;
//! ```

use crate::ast::Parser;
use crate::module::{FfiModuleError, Module};
use crate::types::{function_param_to_ffi, return_type_to_data_type, FfiInterfaceDef, FfiInterfaceMethod};

/// Builder for registering native interface types.
///
/// Created by calling `Module::register_interface(name)`.
///
/// Interfaces define abstract method signatures that classes can implement.
/// Interface methods have no native implementation - they define a contract
/// that script classes must fulfill.
///
/// # Type Parameters
///
/// - `'m`: Lifetime of the mutable borrow of the Module
/// - `'app`: Application lifetime for global property references
#[derive(Debug)]
pub struct InterfaceBuilder<'m, 'app> {
    /// Reference to the module where the interface will be registered
    module: &'m mut Module<'app>,
    /// Interface name
    name: String,
    /// Abstract method signatures (owned, no arena lifetime)
    methods: Vec<FfiInterfaceMethod>,
}

impl<'m, 'app> InterfaceBuilder<'m, 'app> {
    /// Create a new InterfaceBuilder for the given interface name.
    ///
    /// This is called internally by `Module::register_interface()`.
    pub(crate) fn new(module: &'m mut Module<'app>, name: String) -> Self {
        Self {
            module,
            name,
            methods: Vec::new(),
        }
    }

    /// Add an interface method using a declaration string.
    ///
    /// Interface methods are abstract - they define signatures that script
    /// classes must implement. The declaration string specifies the method
    /// signature including return type, name, parameters, and constness.
    ///
    /// # Declaration Format
    ///
    /// ```text
    /// ReturnType name(params) [const]
    /// ```
    ///
    /// # Parameters
    ///
    /// - `decl`: Method declaration (e.g., `"void draw() const"`)
    ///
    /// # Example
    ///
    /// ```ignore
    /// module.register_interface("IGameEntity")
    ///     .method("string getName() const")?
    ///     .method("void setName(const string &in name)")?
    ///     .method("Vec3 getPosition() const")?
    ///     .method("void setPosition(const Vec3 &in pos)")?
    ///     .method("void update(float deltaTime)")?
    ///     .method("void render() const")?
    ///     .build()?;
    /// ```
    pub fn method(mut self, decl: &str) -> Result<Self, FfiModuleError> {
        let method = self.parse_method_decl(decl)?;

        // Check for duplicate method names
        if self.methods.iter().any(|m| m.name == method.name) {
            return Err(FfiModuleError::DuplicateRegistration {
                name: method.name.clone(),
                kind: "interface method".to_string(),
            });
        }

        self.methods.push(method);
        Ok(self)
    }

    /// Finish building and register the interface with the module.
    ///
    /// This consumes the builder and adds the interface definition to the module.
    ///
    /// # Errors
    ///
    /// Returns an error if the interface has no methods.
    pub fn build(self) -> Result<(), FfiModuleError> {
        if self.methods.is_empty() {
            return Err(FfiModuleError::InvalidDeclaration(format!(
                "interface '{}' has no methods",
                self.name
            )));
        }

        // Compute the qualified name and type hash
        let qualified_name = self.module.qualified_name(&self.name);
        let type_hash = crate::types::TypeHash::from_name(&qualified_name);

        let interface_def = FfiInterfaceDef::new(type_hash, self.name, self.methods);

        self.module.add_interface(interface_def);
        Ok(())
    }

    // =========================================================================
    // Internal helpers
    // =========================================================================

    /// Parse a method declaration and convert to FfiInterfaceMethod.
    fn parse_method_decl(&self, decl: &str) -> Result<FfiInterfaceMethod, FfiModuleError> {
        let decl = decl.trim();
        if decl.is_empty() {
            return Err(FfiModuleError::InvalidDeclaration(
                "empty declaration".to_string(),
            ));
        }

        // Parse the declaration using the module's arena
        let sig = Parser::function_decl(decl, self.module.arena()).map_err(|errors| {
            FfiModuleError::InvalidDeclaration(format!("parse error: {}", errors))
        })?;

        // Convert to owned FfiInterfaceMethod
        let params = sig.params.iter().map(function_param_to_ffi).collect();
        let return_type = return_type_to_data_type(&sig.return_type);

        Ok(FfiInterfaceMethod::new(
            sig.name.name.to_string(),
            params,
            return_type,
            sig.is_const,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interface_builder_simple() {
        let mut module = Module::root();
        module
            .register_interface("IDrawable")
            .method("void draw() const")
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(module.interfaces().len(), 1);
        assert_eq!(module.interfaces()[0].name(), "IDrawable");
        assert_eq!(module.interfaces()[0].methods().len(), 1);
        assert_eq!(module.interfaces()[0].methods()[0].name, "draw");
        assert!(module.interfaces()[0].methods()[0].is_const);
    }

    #[test]
    fn interface_builder_multiple_methods() {
        let mut module = Module::root();
        module
            .register_interface("ISerializable")
            .method("string serialize() const")
            .unwrap()
            .method("void deserialize(const string &in data)")
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(module.interfaces().len(), 1);
        assert_eq!(module.interfaces()[0].methods().len(), 2);
        assert_eq!(module.interfaces()[0].methods()[0].name, "serialize");
        assert!(module.interfaces()[0].methods()[0].is_const);
        assert_eq!(module.interfaces()[0].methods()[1].name, "deserialize");
        assert!(!module.interfaces()[0].methods()[1].is_const);
    }

    #[test]
    fn interface_builder_with_params() {
        let mut module = Module::root();
        module
            .register_interface("IGameEntity")
            .method("string getName() const")
            .unwrap()
            .method("void setName(const string &in name)")
            .unwrap()
            .method("void update(float deltaTime)")
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(module.interfaces()[0].methods().len(), 3);

        // Check getName
        assert_eq!(module.interfaces()[0].methods()[0].params.len(), 0);

        // Check setName - has 1 param
        assert_eq!(module.interfaces()[0].methods()[1].params.len(), 1);

        // Check update - has 1 param
        assert_eq!(module.interfaces()[0].methods()[2].params.len(), 1);
    }

    #[test]
    fn interface_builder_empty_interface() {
        let mut module = Module::root();
        let result = module.register_interface("IEmpty").build();

        assert!(result.is_err());
        match result.unwrap_err() {
            FfiModuleError::InvalidDeclaration(msg) => {
                assert!(msg.contains("IEmpty"));
                assert!(msg.contains("no methods"));
            }
            _ => panic!("Expected InvalidDeclaration error"),
        }
    }

    #[test]
    fn interface_builder_invalid_decl() {
        let mut module = Module::root();
        let result = module
            .register_interface("IInvalid")
            .method("not a valid declaration");

        assert!(result.is_err());
    }

    #[test]
    fn interface_builder_empty_decl() {
        let mut module = Module::root();
        let result = module.register_interface("IInvalid").method("");

        assert!(result.is_err());
    }

    #[test]
    fn interface_builder_duplicate_method() {
        let mut module = Module::root();
        let result = module
            .register_interface("IDuplicate")
            .method("void foo()")
            .unwrap()
            .method("void foo()"); // Duplicate!

        assert!(result.is_err());
        match result.unwrap_err() {
            FfiModuleError::DuplicateRegistration { name, kind } => {
                assert_eq!(name, "foo");
                assert_eq!(kind, "interface method");
            }
            _ => panic!("Expected DuplicateRegistration error"),
        }
    }

    #[test]
    fn interface_builder_const_and_non_const() {
        let mut module = Module::root();
        module
            .register_interface("IAccessor")
            .method("int getValue() const")
            .unwrap()
            .method("void setValue(int value)")
            .unwrap()
            .build()
            .unwrap();

        assert!(module.interfaces()[0].methods()[0].is_const);
        assert!(!module.interfaces()[0].methods()[1].is_const);
    }

    #[test]
    fn interface_builder_complex_return_types() {
        let mut module = Module::root();
        module
            .register_interface("IFactory")
            .method("Entity@ createEntity()")
            .unwrap()
            .method("array<string>@ getNames() const")
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(module.interfaces()[0].methods().len(), 2);
    }

    #[test]
    fn interface_builder_multiple_interfaces() {
        let mut module = Module::root();

        module
            .register_interface("IFirst")
            .method("void first()")
            .unwrap()
            .build()
            .unwrap();

        module
            .register_interface("ISecond")
            .method("void second()")
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(module.interfaces().len(), 2);
        assert_eq!(module.interfaces()[0].name(), "IFirst");
        assert_eq!(module.interfaces()[1].name(), "ISecond");
    }
}
