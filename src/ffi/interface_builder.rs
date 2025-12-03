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
use crate::ffi::{NativeInterfaceDef, NativeInterfaceMethod};
use crate::module::{FfiModuleError, Module};
use crate::semantic::types::type_def::TypeId;

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
    /// Abstract method signatures
    methods: Vec<NativeInterfaceMethod<'static>>,
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
        let sig = self.parse_method_decl(decl)?;

        // Check for duplicate method names
        if self.methods.iter().any(|m| m.name.name == sig.name.name) {
            return Err(FfiModuleError::DuplicateRegistration {
                name: sig.name.name.to_string(),
                kind: "interface method".to_string(),
            });
        }

        let method = NativeInterfaceMethod {
            name: sig.name,
            params: sig.params,
            return_type: sig.return_type,
            is_const: sig.is_const,
        };

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

        let interface_def = NativeInterfaceDef {
            id: TypeId::next(),
            name: self.name,
            methods: self.methods,
        };

        self.module.add_interface(interface_def);
        Ok(())
    }

    // =========================================================================
    // Internal helpers
    // =========================================================================

    /// Parse a method declaration using the module's arena.
    fn parse_method_decl(
        &self,
        decl: &str,
    ) -> Result<crate::ast::FunctionSignatureDecl<'static>, FfiModuleError> {
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

        // SAFETY: The arena is owned by module and lives as long as module.
        // We transmute the lifetime to 'static for storage.
        Ok(unsafe { std::mem::transmute(sig) })
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
        assert_eq!(module.interfaces()[0].name, "IDrawable");
        assert_eq!(module.interfaces()[0].methods.len(), 1);
        assert_eq!(module.interfaces()[0].methods[0].name.name, "draw");
        assert!(module.interfaces()[0].methods[0].is_const);
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
        assert_eq!(module.interfaces()[0].methods.len(), 2);
        assert_eq!(module.interfaces()[0].methods[0].name.name, "serialize");
        assert!(module.interfaces()[0].methods[0].is_const);
        assert_eq!(module.interfaces()[0].methods[1].name.name, "deserialize");
        assert!(!module.interfaces()[0].methods[1].is_const);
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

        assert_eq!(module.interfaces()[0].methods.len(), 3);

        // Check getName
        assert_eq!(module.interfaces()[0].methods[0].params.len(), 0);

        // Check setName - has 1 param
        assert_eq!(module.interfaces()[0].methods[1].params.len(), 1);

        // Check update - has 1 param
        assert_eq!(module.interfaces()[0].methods[2].params.len(), 1);
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

        assert!(module.interfaces()[0].methods[0].is_const);
        assert!(!module.interfaces()[0].methods[1].is_const);
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

        assert_eq!(module.interfaces()[0].methods.len(), 2);
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
        assert_eq!(module.interfaces()[0].name, "IFirst");
        assert_eq!(module.interfaces()[1].name, "ISecond");
    }
}
