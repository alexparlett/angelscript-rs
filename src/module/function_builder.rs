//! Function builder for FFI registration.
//!
//! This module provides `FunctionBuilder`, a builder pattern for creating
//! `FunctionDef` instances that can be registered with the FFI system.
//!
//! # Example
//!
//! ```ignore
//! use angelscript_core::{FunctionDef, Param, DataType, primitives};
//!
//! let func = FunctionBuilder::new("process")
//!     .with_params(vec![
//!         Param::new("obj", DataType::handle(TypeHash::from_name("MyClass"))),
//!     ])
//!     .with_return_type(DataType::simple(primitives::VOID))
//!     .build();
//! ```

use angelscript_ffi::NativeFn;
use angelscript_core::{DataType, FunctionDef, FunctionTraits, OperatorBehavior, Param, TypeHash, Visibility};

/// Builder for creating `FunctionDef` instances.
///
/// This provides a convenient builder pattern for constructing FFI function
/// definitions with proper hash computation.
#[derive(Debug)]
pub struct FunctionBuilder {
    /// Function name (unqualified)
    pub name: String,

    /// Namespace path (e.g., ["Game", "Player"])
    pub namespace: Vec<String>,

    /// Parameters with types and optional defaults
    pub params: Vec<Param>,

    /// Return type (always resolved)
    pub return_type: DataType,

    /// Function traits (const, virtual, constructor, etc.)
    pub traits: FunctionTraits,

    /// Owning type for methods (None for global functions)
    pub owner_type: Option<TypeHash>,

    /// Operator behavior if this is an operator method
    pub operator: Option<OperatorBehavior>,

    /// Visibility (public, private, protected)
    pub visibility: Visibility,

    /// The native function implementation
    pub native_fn: Option<NativeFn>,

    /// Computed function hash (identity) based on name + params + owner
    pub func_hash: TypeHash,
}

impl FunctionBuilder {
    /// Create a new function builder.
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let func_hash = TypeHash::from_function(&name, &[]);
        Self {
            name,
            namespace: Vec::new(),
            params: Vec::new(),
            return_type: DataType::simple(angelscript_core::primitives::VOID),
            traits: FunctionTraits::new(),
            owner_type: None,
            operator: None,
            visibility: Visibility::Public,
            native_fn: None,
            func_hash,
        }
    }

    /// Recompute the function hash based on current name, params, and owner.
    ///
    /// This is called automatically by builder methods, but exposed publicly
    /// for cases where you need to update the hash after direct field mutation.
    pub fn recompute_hash_pub(&mut self) {
        self.recompute_hash();
    }

    /// Recompute the function hash based on current name, params, and owner.
    fn recompute_hash(&mut self) {
        let param_hashes: Vec<TypeHash> = self.params.iter().map(|p| p.data_type.type_hash).collect();
        let is_const = self.traits.is_const;
        let return_is_const = self.return_type.is_const;

        self.func_hash = if let Some(owner) = self.owner_type {
            if self.traits.is_constructor {
                TypeHash::from_constructor(owner, &param_hashes)
            } else if self.operator.is_some() {
                TypeHash::from_operator(owner, &self.name, &param_hashes, is_const, return_is_const)
            } else {
                TypeHash::from_method(owner, &self.name, &param_hashes, is_const, return_is_const)
            }
        } else {
            TypeHash::from_function(&self.qualified_name(), &param_hashes)
        };
    }

    /// Get the qualified name of this function (with namespace).
    pub fn qualified_name(&self) -> String {
        if self.namespace.is_empty() {
            self.name.clone()
        } else {
            format!("{}::{}", self.namespace.join("::"), self.name)
        }
    }

    /// Check if this is a method (has an owner type).
    pub fn is_method(&self) -> bool {
        self.owner_type.is_some()
    }

    /// Check if this is a global function.
    pub fn is_global(&self) -> bool {
        self.owner_type.is_none()
    }

    /// Check if this is a constructor.
    pub fn is_constructor(&self) -> bool {
        self.traits.is_constructor
    }

    /// Check if this is a destructor.
    pub fn is_destructor(&self) -> bool {
        self.traits.is_destructor
    }

    /// Check if this is a const method.
    pub fn is_const(&self) -> bool {
        self.traits.is_const
    }

    /// Check if this is an operator method.
    pub fn is_operator(&self) -> bool {
        self.operator.is_some()
    }

    /// Get the number of required parameters (without defaults).
    pub fn required_param_count(&self) -> usize {
        self.params.iter().filter(|p| !p.has_default).count()
    }

    /// Get the total number of parameters.
    pub fn param_count(&self) -> usize {
        self.params.len()
    }

    /// Set the namespace for this function.
    pub fn with_namespace(mut self, namespace: Vec<String>) -> Self {
        self.namespace = namespace;
        self.recompute_hash();
        self
    }

    /// Set the parameters for this function.
    pub fn with_params(mut self, params: Vec<Param>) -> Self {
        self.params = params;
        self.recompute_hash();
        self
    }

    /// Set the return type for this function.
    pub fn with_return_type(mut self, return_type: DataType) -> Self {
        self.return_type = return_type;
        self
    }

    /// Set the function traits.
    pub fn with_traits(mut self, traits: FunctionTraits) -> Self {
        self.traits = traits;
        self.recompute_hash();
        self
    }

    /// Set the owner type for methods.
    pub fn with_owner_type(mut self, owner_type: TypeHash) -> Self {
        self.owner_type = Some(owner_type);
        self.recompute_hash();
        self
    }

    /// Set the operator behavior.
    pub fn with_operator(mut self, operator: OperatorBehavior) -> Self {
        self.operator = Some(operator);
        self.recompute_hash();
        self
    }

    /// Set the visibility.
    pub fn with_visibility(mut self, visibility: Visibility) -> Self {
        self.visibility = visibility;
        self
    }

    /// Set the native function implementation.
    pub fn with_native_fn(mut self, native_fn: NativeFn) -> Self {
        self.native_fn = Some(native_fn);
        self
    }

    /// Set whether this is a const method.
    pub fn with_const(mut self, is_const: bool) -> Self {
        self.traits.is_const = is_const;
        self
    }

    /// Build a `FunctionDef` from this builder.
    pub fn build(self) -> FunctionDef {
        FunctionDef {
            func_hash: self.func_hash,
            name: self.name,
            namespace: self.namespace,
            params: self.params,
            return_type: self.return_type,
            object_type: self.owner_type,
            traits: self.traits,
            is_native: true,
            visibility: self.visibility,
        }
    }

    /// Build a `FunctionDef` and return the native function separately.
    ///
    /// This is useful for registering with `FfiRegistryBuilder` which stores
    /// the native function separately.
    pub fn build_with_native(mut self) -> (FunctionDef, Option<NativeFn>) {
        let native_fn = self.native_fn.take();
        (self.build(), native_fn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::primitives;

    #[test]
    fn function_builder_new() {
        let builder = FunctionBuilder::new("test");
        assert_eq!(builder.name, "test");
        assert!(builder.namespace.is_empty());
        assert!(builder.params.is_empty());
        assert!(!builder.is_method());
        assert!(builder.is_global());
    }

    #[test]
    fn function_builder_qualified_name() {
        let builder = FunctionBuilder::new("test");
        assert_eq!(builder.qualified_name(), "test");

        let builder_ns = builder.with_namespace(vec!["Game".to_string(), "Player".to_string()]);
        assert_eq!(builder_ns.qualified_name(), "Game::Player::test");
    }

    #[test]
    fn function_builder_with_params() {
        let builder = FunctionBuilder::new("add")
            .with_params(vec![
                Param::new("a", DataType::simple(primitives::INT32)),
                Param::new("b", DataType::simple(primitives::INT32)),
            ])
            .with_return_type(DataType::simple(primitives::INT32));

        assert_eq!(builder.param_count(), 2);
        assert_eq!(builder.required_param_count(), 2);
    }

    #[test]
    fn function_builder_with_defaults() {
        let builder = FunctionBuilder::new("greet")
            .with_params(vec![
                Param::new("name", DataType::simple(primitives::STRING)),
                Param::with_default("greeting", DataType::simple(primitives::STRING)),
            ]);

        assert_eq!(builder.param_count(), 2);
        assert_eq!(builder.required_param_count(), 1);
    }

    #[test]
    fn function_builder_method() {
        let owner_type = TypeHash(100);
        let builder = FunctionBuilder::new("getValue")
            .with_owner_type(owner_type)
            .with_traits(FunctionTraits {
                is_const: true,
                ..FunctionTraits::new()
            });

        assert!(builder.is_method());
        assert!(!builder.is_global());
        assert!(builder.is_const());
        assert_eq!(builder.owner_type, Some(owner_type));
    }

    #[test]
    fn function_builder_constructor() {
        let owner_type = TypeHash(100);
        let builder = FunctionBuilder::new("MyClass")
            .with_owner_type(owner_type)
            .with_traits(FunctionTraits {
                is_constructor: true,
                ..FunctionTraits::new()
            });

        assert!(builder.is_constructor());
        assert!(!builder.is_destructor());
    }

    #[test]
    fn function_builder_operator() {
        let owner_type = TypeHash(100);
        let builder = FunctionBuilder::new("opAdd")
            .with_owner_type(owner_type)
            .with_operator(OperatorBehavior::OpAdd);

        assert!(builder.is_operator());
        assert_eq!(builder.operator, Some(OperatorBehavior::OpAdd));
    }

    #[test]
    fn function_builder_build() {
        let builder = FunctionBuilder::new("add")
            .with_params(vec![
                Param::new("a", DataType::simple(primitives::INT32)),
                Param::new("b", DataType::simple(primitives::INT32)),
            ])
            .with_return_type(DataType::simple(primitives::INT32));

        let func_def = builder.build();

        assert_eq!(func_def.name, "add");
        assert_eq!(func_def.params.len(), 2);
        assert_eq!(func_def.params[0].data_type.type_hash, primitives::INT32);
        assert_eq!(func_def.params[1].data_type.type_hash, primitives::INT32);
        assert_eq!(func_def.return_type.type_hash, primitives::INT32);
        assert!(func_def.is_native);
    }

    #[test]
    fn function_builder_with_user_type() {
        let my_class_hash = TypeHash::from_name("MyClass");

        let builder = FunctionBuilder::new("process")
            .with_params(vec![Param::new(
                "obj",
                DataType::with_handle(my_class_hash, false),
            )])
            .with_return_type(DataType::simple(primitives::VOID));

        let func_def = builder.build();

        assert_eq!(func_def.params[0].data_type.type_hash, my_class_hash);
        assert!(func_def.params[0].data_type.is_handle);
    }
}
