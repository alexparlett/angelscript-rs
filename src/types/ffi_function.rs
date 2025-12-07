//! Owned function definitions for FFI registry.
//!
//! This module provides `FfiFunctionDef` and `FfiParam`, which are owned
//! function definitions that can be stored in `Arc<FfiRegistry>` without
//! arena lifetimes.
//!
//! All types are resolved immediately using deterministic `TypeHash`:
//! - Primitives use their well-known hashes
//! - User types use `TypeHash::from_name()`
//! - Template parameters use `TypeHash::SELF`
//!
//! # Example
//!
//! ```ignore
//! // Create an FFI function definition with already-resolved types
//! let func = FfiFunctionDef::new("process")
//!     .with_params(vec![
//!         FfiParam::new("obj", DataType::handle(TypeHash::from_name("MyClass"))),
//!     ])
//!     .with_return_type(DataType::simple(primitive_hashes::VOID));
//!
//! // func_hash is computed from name + params
//! ```

use crate::ffi::NativeFn;
use crate::semantic::types::type_def::{FunctionTraits, OperatorBehavior, Visibility};
use crate::semantic::types::DataType;
use crate::types::TypeHash;

use super::ffi_expr::FfiExpr;

/// A parameter in an FFI function definition.
///
/// Uses `DataType` for resolved types and `FfiExpr` for
/// owned default argument values.
#[derive(Debug, Clone, PartialEq)]
pub struct FfiParam {
    /// Parameter name
    pub name: String,

    /// Parameter type (always resolved)
    pub data_type: DataType,

    /// Default value expression (if any)
    pub default_value: Option<FfiExpr>,
}

impl FfiParam {
    /// Create a new parameter with no default value.
    pub fn new(name: impl Into<String>, data_type: DataType) -> Self {
        Self {
            name: name.into(),
            data_type,
            default_value: None,
        }
    }

    /// Create a new parameter with a default value.
    pub fn with_default(
        name: impl Into<String>,
        data_type: DataType,
        default_value: FfiExpr,
    ) -> Self {
        Self {
            name: name.into(),
            data_type,
            default_value: Some(default_value),
        }
    }

    /// Check if this parameter has a default value.
    pub fn has_default(&self) -> bool {
        self.default_value.is_some()
    }
}

/// Owned function definition for FFI registry.
///
/// This is the FFI equivalent of `FunctionDef<'ast>`, but fully owned
/// so it can be stored in `Arc<FfiRegistry>`.
///
/// All type references use `DataType` with deterministic `TypeHash` values.
#[derive(Debug)]
pub struct FfiFunctionDef {
    /// Function name (unqualified)
    pub name: String,

    /// Namespace path (e.g., ["Game", "Player"])
    pub namespace: Vec<String>,

    /// Parameters with types and optional defaults
    pub params: Vec<FfiParam>,

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

impl FfiFunctionDef {
    /// Create a new FFI function definition.
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let func_hash = TypeHash::from_function(&name, &[]);
        Self {
            name,
            namespace: Vec::new(),
            params: Vec::new(),
            return_type: DataType::simple(crate::types::primitive_hashes::VOID),
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
        self.params.iter().filter(|p| !p.has_default()).count()
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
    pub fn with_params(mut self, params: Vec<FfiParam>) -> Self {
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

    /// Convert to a ResolvedFfiFunctionDef.
    ///
    /// Since all types are now resolved immediately, this is just a conversion
    /// that strips the native_fn field (stored separately in the registry).
    pub fn to_resolved(&self) -> ResolvedFfiFunctionDef {
        ResolvedFfiFunctionDef {
            name: self.name.clone(),
            namespace: self.namespace.clone(),
            params: self.params.iter().map(|p| ResolvedFfiParam {
                name: p.name.clone(),
                data_type: p.data_type.clone(),
                default_value: p.default_value.clone(),
            }).collect(),
            return_type: self.return_type.clone(),
            traits: self.traits,
            owner_type: self.owner_type,
            operator: self.operator,
            visibility: self.visibility,
            func_hash: self.func_hash,
        }
    }
}

/// A resolved FFI parameter with concrete `DataType`.
///
/// Now identical to `FfiParam` since all types are resolved immediately.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedFfiParam {
    /// Parameter name
    pub name: String,

    /// Resolved parameter type
    pub data_type: DataType,

    /// Default value expression (if any)
    /// Note: This is still `FfiExpr` because evaluation happens at call time
    pub default_value: Option<FfiExpr>,
}

/// A fully resolved FFI function definition.
///
/// Now essentially identical to `FfiFunctionDef` (minus `native_fn` which is
/// stored separately). Kept for API compatibility.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedFfiFunctionDef {
    /// Function name (unqualified)
    pub name: String,

    /// Namespace path
    pub namespace: Vec<String>,

    /// Resolved parameters
    pub params: Vec<ResolvedFfiParam>,

    /// Resolved return type
    pub return_type: DataType,

    /// Function traits
    pub traits: FunctionTraits,

    /// Owning type for methods
    pub owner_type: Option<TypeHash>,

    /// Operator behavior if this is an operator method
    pub operator: Option<OperatorBehavior>,

    /// Visibility
    pub visibility: Visibility,

    /// Deterministic hash for this function (computed from name + parameter types)
    pub func_hash: TypeHash,
}

impl ResolvedFfiFunctionDef {
    /// Get the qualified name of this function.
    pub fn qualified_name(&self) -> String {
        if self.namespace.is_empty() {
            self.name.clone()
        } else {
            format!("{}::{}", self.namespace.join("::"), self.name)
        }
    }

    /// Get parameter types as a slice of DataType.
    pub fn param_types(&self) -> Vec<DataType> {
        self.params.iter().map(|p| p.data_type.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::primitive_hashes;

    #[test]
    fn ffi_param_new() {
        let param = FfiParam::new("x", DataType::simple(primitive_hashes::INT32));
        assert_eq!(param.name, "x");
        assert!(!param.has_default());
    }

    #[test]
    fn ffi_param_with_default() {
        let param = FfiParam::with_default(
            "x",
            DataType::simple(primitive_hashes::INT32),
            FfiExpr::int(42),
        );
        assert_eq!(param.name, "x");
        assert!(param.has_default());
        assert_eq!(param.default_value, Some(FfiExpr::int(42)));
    }

    #[test]
    fn ffi_function_def_new() {
        let func = FfiFunctionDef::new("test");
        assert_eq!(func.name, "test");
        assert!(func.namespace.is_empty());
        assert!(func.params.is_empty());
        assert!(!func.is_method());
        assert!(func.is_global());
    }

    #[test]
    fn ffi_function_def_qualified_name() {
        let func = FfiFunctionDef::new("test");
        assert_eq!(func.qualified_name(), "test");

        let func_ns = func.with_namespace(vec!["Game".to_string(), "Player".to_string()]);
        assert_eq!(func_ns.qualified_name(), "Game::Player::test");
    }

    #[test]
    fn ffi_function_def_with_params() {
        let func = FfiFunctionDef::new("add")
            .with_params(vec![
                FfiParam::new("a", DataType::simple(primitive_hashes::INT32)),
                FfiParam::new("b", DataType::simple(primitive_hashes::INT32)),
            ])
            .with_return_type(DataType::simple(primitive_hashes::INT32));

        assert_eq!(func.param_count(), 2);
        assert_eq!(func.required_param_count(), 2);
    }

    #[test]
    fn ffi_function_def_with_defaults() {
        let func = FfiFunctionDef::new("greet")
            .with_params(vec![
                FfiParam::new("name", DataType::simple(primitive_hashes::STRING)),
                FfiParam::with_default(
                    "greeting",
                    DataType::simple(primitive_hashes::STRING),
                    FfiExpr::string("Hello"),
                ),
            ]);

        assert_eq!(func.param_count(), 2);
        assert_eq!(func.required_param_count(), 1);
    }

    #[test]
    fn ffi_function_def_method() {
        let owner_type = TypeHash(100);
        let func = FfiFunctionDef::new("getValue")
            .with_owner_type(owner_type)
            .with_traits(FunctionTraits {
                is_const: true,
                ..FunctionTraits::new()
            });

        assert!(func.is_method());
        assert!(!func.is_global());
        assert!(func.is_const());
        assert_eq!(func.owner_type, Some(owner_type));
    }

    #[test]
    fn ffi_function_def_constructor() {
        let owner_type = TypeHash(100);
        let func = FfiFunctionDef::new("MyClass")
            .with_owner_type(owner_type)
            .with_traits(FunctionTraits {
                is_constructor: true,
                ..FunctionTraits::new()
            });

        assert!(func.is_constructor());
        assert!(!func.is_destructor());
    }

    #[test]
    fn ffi_function_def_operator() {
        let owner_type = TypeHash(100);
        let func = FfiFunctionDef::new("opAdd")
            .with_owner_type(owner_type)
            .with_operator(OperatorBehavior::OpAdd);

        assert!(func.is_operator());
        assert_eq!(func.operator, Some(OperatorBehavior::OpAdd));
    }

    #[test]
    fn ffi_function_def_to_resolved() {
        let func = FfiFunctionDef::new("add")
            .with_params(vec![
                FfiParam::new("a", DataType::simple(primitive_hashes::INT32)),
                FfiParam::new("b", DataType::simple(primitive_hashes::INT32)),
            ])
            .with_return_type(DataType::simple(primitive_hashes::INT32));

        let resolved = func.to_resolved();

        assert_eq!(resolved.name, "add");
        assert_eq!(resolved.params.len(), 2);
        assert_eq!(resolved.params[0].data_type.type_hash, primitive_hashes::INT32);
        assert_eq!(resolved.params[1].data_type.type_hash, primitive_hashes::INT32);
        assert_eq!(resolved.return_type.type_hash, primitive_hashes::INT32);
    }

    #[test]
    fn ffi_function_def_with_user_type() {
        // User types are resolved immediately using TypeHash::from_name()
        let my_class_hash = TypeHash::from_name("MyClass");

        let func = FfiFunctionDef::new("process")
            .with_params(vec![FfiParam::new(
                "obj",
                DataType::with_handle(my_class_hash, false),
            )])
            .with_return_type(DataType::simple(primitive_hashes::VOID));

        let resolved = func.to_resolved();

        assert_eq!(resolved.params[0].data_type.type_hash, my_class_hash);
        assert!(resolved.params[0].data_type.is_handle);
    }

    #[test]
    fn resolved_ffi_function_def_param_types() {
        let resolved = ResolvedFfiFunctionDef {
            name: "add".to_string(),
            namespace: Vec::new(),
            params: vec![
                ResolvedFfiParam {
                    name: "a".to_string(),
                    data_type: DataType::simple(primitive_hashes::INT32),
                    default_value: None,
                },
                ResolvedFfiParam {
                    name: "b".to_string(),
                    data_type: DataType::simple(primitive_hashes::INT32),
                    default_value: None,
                },
            ],
            return_type: DataType::simple(primitive_hashes::INT32),
            traits: FunctionTraits::new(),
            owner_type: None,
            operator: None,
            visibility: Visibility::Public,
            func_hash: TypeHash::from_function("add", &[primitive_hashes::INT32, primitive_hashes::INT32]),
        };

        let param_types = resolved.param_types();
        assert_eq!(param_types.len(), 2);
        assert_eq!(param_types[0].type_hash, primitive_hashes::INT32);
        assert_eq!(param_types[1].type_hash, primitive_hashes::INT32);
    }

    #[test]
    fn resolved_ffi_function_def_qualified_name() {
        let resolved = ResolvedFfiFunctionDef {
            name: "test".to_string(),
            namespace: vec!["Game".to_string()],
            params: Vec::new(),
            return_type: DataType::simple(primitive_hashes::VOID),
            traits: FunctionTraits::new(),
            owner_type: None,
            operator: None,
            visibility: Visibility::Public,
            func_hash: TypeHash::from_function("Game::test", &[]),
        };

        assert_eq!(resolved.qualified_name(), "Game::test");
    }

    #[test]
    fn ffi_param_clone() {
        let param = FfiParam::with_default(
            "x",
            DataType::simple(primitive_hashes::INT32),
            FfiExpr::int(42),
        );

        let cloned = param.clone();
        assert_eq!(param, cloned);
    }

    #[test]
    fn resolved_ffi_param_clone() {
        let param = ResolvedFfiParam {
            name: "x".to_string(),
            data_type: DataType::simple(primitive_hashes::INT32),
            default_value: Some(FfiExpr::int(42)),
        };

        let cloned = param.clone();
        assert_eq!(param, cloned);
    }
}
