//! Owned function definitions for FFI registry.
//!
//! This module provides `FfiFunctionDef` and `FfiParam`, which are owned
//! function definitions that can be stored in `Arc<FfiRegistry>` without
//! arena lifetimes.
//!
//! # Problem
//!
//! The current `FunctionDef<'ast>` has arena lifetimes because:
//! - `default_args: Vec<Option<&'ast Expr<'ast>>>` - default argument expressions
//!
//! For FFI functions to be stored in a shared registry, we need fully owned types.
//!
//! # Solution
//!
//! - `FfiParam` uses `FfiDataType` for deferred type resolution
//! - `FfiParam` uses `FfiExpr` for default argument values
//! - `FfiFunctionDef` holds owned data that can be resolved later
//!
//! # Example
//!
//! ```ignore
//! // Create an FFI function definition
//! let func = FfiFunctionDef {
//!     id: FunctionId::next_ffi(),
//!     name: "process".to_string(),
//!     params: vec![
//!         FfiParam {
//!             name: "obj".to_string(),
//!             data_type: FfiDataType::unresolved_handle("MyClass", false),
//!             default_value: None,
//!         },
//!     ],
//!     return_type: FfiDataType::resolved(DataType::simple(VOID_TYPE)),
//!     ..Default::default()
//! };
//!
//! // Resolve during Context sealing
//! let resolved = func.resolve(&lookup, &mut instantiate)?;
//! ```

use crate::ffi::NativeFn;
use crate::semantic::types::type_def::{FunctionId, FunctionTraits, OperatorBehavior, TypeId, Visibility};
use crate::semantic::types::DataType;
use crate::types::FfiDataType;

use super::ffi_expr::FfiExpr;

/// A parameter in an FFI function definition.
///
/// Uses `FfiDataType` for deferred type resolution and `FfiExpr` for
/// owned default argument values.
#[derive(Debug, Clone, PartialEq)]
pub struct FfiParam {
    /// Parameter name
    pub name: String,

    /// Parameter type (may be unresolved)
    pub data_type: FfiDataType,

    /// Default value expression (if any)
    pub default_value: Option<FfiExpr>,
}

impl FfiParam {
    /// Create a new parameter with no default value.
    pub fn new(name: impl Into<String>, data_type: FfiDataType) -> Self {
        Self {
            name: name.into(),
            data_type,
            default_value: None,
        }
    }

    /// Create a new parameter with a default value.
    pub fn with_default(
        name: impl Into<String>,
        data_type: FfiDataType,
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
/// Type references are stored as `FfiDataType` which may be unresolved
/// during registration and resolved during Context sealing.
#[derive(Debug)]
pub struct FfiFunctionDef {
    /// Unique function ID
    pub id: FunctionId,

    /// Function name (unqualified)
    pub name: String,

    /// Namespace path (e.g., ["Game", "Player"])
    pub namespace: Vec<String>,

    /// Parameters with types and optional defaults
    pub params: Vec<FfiParam>,

    /// Return type (may be unresolved)
    pub return_type: FfiDataType,

    /// Function traits (const, virtual, constructor, etc.)
    pub traits: FunctionTraits,

    /// Owning type for methods (None for global functions)
    pub owner_type: Option<TypeId>,

    /// Operator behavior if this is an operator method
    pub operator: Option<OperatorBehavior>,

    /// Visibility (public, private, protected)
    pub visibility: Visibility,

    /// The native function implementation
    pub native_fn: Option<NativeFn>,
}

impl FfiFunctionDef {
    /// Create a new FFI function definition.
    pub fn new(id: FunctionId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            namespace: Vec::new(),
            params: Vec::new(),
            return_type: FfiDataType::resolved(DataType::simple(
                crate::semantic::types::type_def::VOID_TYPE,
            )),
            traits: FunctionTraits::new(),
            owner_type: None,
            operator: None,
            visibility: Visibility::Public,
            native_fn: None,
        }
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
        self
    }

    /// Set the parameters for this function.
    pub fn with_params(mut self, params: Vec<FfiParam>) -> Self {
        self.params = params;
        self
    }

    /// Set the return type for this function.
    pub fn with_return_type(mut self, return_type: FfiDataType) -> Self {
        self.return_type = return_type;
        self
    }

    /// Set the function traits.
    pub fn with_traits(mut self, traits: FunctionTraits) -> Self {
        self.traits = traits;
        self
    }

    /// Set the owner type for methods.
    pub fn with_owner_type(mut self, owner_type: TypeId) -> Self {
        self.owner_type = Some(owner_type);
        self
    }

    /// Set the operator behavior.
    pub fn with_operator(mut self, operator: OperatorBehavior) -> Self {
        self.operator = Some(operator);
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

    /// Resolve all `FfiDataType` references to concrete `DataType`.
    ///
    /// This is called during Context sealing when all types are known.
    ///
    /// # Arguments
    ///
    /// * `lookup` - Function to look up a TypeId by name
    /// * `instantiate` - Function to instantiate templates
    ///
    /// # Returns
    ///
    /// A `ResolvedFfiFunctionDef` with all types resolved, or an error.
    pub fn resolve<L, I>(
        &self,
        lookup: &L,
        instantiate: &mut I,
    ) -> Result<ResolvedFfiFunctionDef, FfiResolutionError>
    where
        L: Fn(&str) -> Option<TypeId>,
        I: FnMut(TypeId, Vec<DataType>) -> Result<TypeId, String>,
    {
        // Resolve return type
        let return_type = self
            .return_type
            .resolve(lookup, instantiate)
            .map_err(|e| FfiResolutionError::ReturnType {
                function: self.qualified_name(),
                error: e,
            })?;

        // Resolve parameter types
        let mut params = Vec::with_capacity(self.params.len());
        for (i, param) in self.params.iter().enumerate() {
            let resolved_type =
                param
                    .data_type
                    .resolve(lookup, instantiate)
                    .map_err(|e| FfiResolutionError::Parameter {
                        function: self.qualified_name(),
                        param_name: param.name.clone(),
                        param_index: i,
                        error: e,
                    })?;

            params.push(ResolvedFfiParam {
                name: param.name.clone(),
                data_type: resolved_type,
                default_value: param.default_value.clone(),
            });
        }

        Ok(ResolvedFfiFunctionDef {
            id: self.id,
            name: self.name.clone(),
            namespace: self.namespace.clone(),
            params,
            return_type,
            traits: self.traits,
            owner_type: self.owner_type,
            operator: self.operator,
            visibility: self.visibility,
        })
    }
}

/// A resolved FFI parameter with concrete `DataType`.
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
/// This is produced by `FfiFunctionDef::resolve()` and contains concrete
/// `DataType` references that can be used for type checking.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedFfiFunctionDef {
    /// Unique function ID
    pub id: FunctionId,

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
    pub owner_type: Option<TypeId>,

    /// Operator behavior if this is an operator method
    pub operator: Option<OperatorBehavior>,

    /// Visibility
    pub visibility: Visibility,
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

/// Errors that can occur during FFI function resolution.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum FfiResolutionError {
    /// Failed to resolve return type
    #[error("failed to resolve return type of function '{function}': {error}")]
    ReturnType {
        /// The function name
        function: String,
        /// The error message
        error: String,
    },

    /// Failed to resolve parameter type
    #[error("failed to resolve parameter '{param_name}' (index {param_index}) of function '{function}': {error}")]
    Parameter {
        /// The function name
        function: String,
        /// The parameter name
        param_name: String,
        /// The parameter index
        param_index: usize,
        /// The error message
        error: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::types::type_def::{INT32_TYPE, VOID_TYPE};

    #[test]
    fn ffi_param_new() {
        let param = FfiParam::new("x", FfiDataType::resolved(DataType::simple(INT32_TYPE)));
        assert_eq!(param.name, "x");
        assert!(!param.has_default());
    }

    #[test]
    fn ffi_param_with_default() {
        let param = FfiParam::with_default(
            "x",
            FfiDataType::resolved(DataType::simple(INT32_TYPE)),
            FfiExpr::int(42),
        );
        assert_eq!(param.name, "x");
        assert!(param.has_default());
        assert_eq!(param.default_value, Some(FfiExpr::int(42)));
    }

    #[test]
    fn ffi_function_def_new() {
        let func = FfiFunctionDef::new(FunctionId::new(1), "test");
        assert_eq!(func.name, "test");
        assert_eq!(func.id, FunctionId::new(1));
        assert!(func.namespace.is_empty());
        assert!(func.params.is_empty());
        assert!(!func.is_method());
        assert!(func.is_global());
    }

    #[test]
    fn ffi_function_def_qualified_name() {
        let func = FfiFunctionDef::new(FunctionId::new(1), "test");
        assert_eq!(func.qualified_name(), "test");

        let func_ns = func.with_namespace(vec!["Game".to_string(), "Player".to_string()]);
        assert_eq!(func_ns.qualified_name(), "Game::Player::test");
    }

    #[test]
    fn ffi_function_def_with_params() {
        let func = FfiFunctionDef::new(FunctionId::new(1), "add")
            .with_params(vec![
                FfiParam::new("a", FfiDataType::resolved(DataType::simple(INT32_TYPE))),
                FfiParam::new("b", FfiDataType::resolved(DataType::simple(INT32_TYPE))),
            ])
            .with_return_type(FfiDataType::resolved(DataType::simple(INT32_TYPE)));

        assert_eq!(func.param_count(), 2);
        assert_eq!(func.required_param_count(), 2);
    }

    #[test]
    fn ffi_function_def_with_defaults() {
        let func = FfiFunctionDef::new(FunctionId::new(1), "greet")
            .with_params(vec![
                FfiParam::new("name", FfiDataType::unresolved_simple("string")),
                FfiParam::with_default(
                    "greeting",
                    FfiDataType::unresolved_simple("string"),
                    FfiExpr::string("Hello"),
                ),
            ]);

        assert_eq!(func.param_count(), 2);
        assert_eq!(func.required_param_count(), 1);
    }

    #[test]
    fn ffi_function_def_method() {
        let owner_type = TypeId::new(100);
        let func = FfiFunctionDef::new(FunctionId::new(1), "getValue")
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
        let owner_type = TypeId::new(100);
        let func = FfiFunctionDef::new(FunctionId::new(1), "MyClass")
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
        let owner_type = TypeId::new(100);
        let func = FfiFunctionDef::new(FunctionId::new(1), "opAdd")
            .with_owner_type(owner_type)
            .with_operator(OperatorBehavior::OpAdd);

        assert!(func.is_operator());
        assert_eq!(func.operator, Some(OperatorBehavior::OpAdd));
    }

    #[test]
    fn ffi_function_def_resolve_simple() {
        let func = FfiFunctionDef::new(FunctionId::new(1), "add")
            .with_params(vec![
                FfiParam::new("a", FfiDataType::resolved(DataType::simple(INT32_TYPE))),
                FfiParam::new("b", FfiDataType::resolved(DataType::simple(INT32_TYPE))),
            ])
            .with_return_type(FfiDataType::resolved(DataType::simple(INT32_TYPE)));

        let lookup = |_: &str| -> Option<TypeId> { None };
        let mut instantiate = |_: TypeId, _: Vec<DataType>| -> Result<TypeId, String> {
            Err("no templates".to_string())
        };

        let resolved = func.resolve(&lookup, &mut instantiate).unwrap();

        assert_eq!(resolved.name, "add");
        assert_eq!(resolved.params.len(), 2);
        assert_eq!(resolved.params[0].data_type.type_id, INT32_TYPE);
        assert_eq!(resolved.params[1].data_type.type_id, INT32_TYPE);
        assert_eq!(resolved.return_type.type_id, INT32_TYPE);
    }

    #[test]
    fn ffi_function_def_resolve_with_unresolved() {
        let my_class_id = TypeId::new(100);

        let func = FfiFunctionDef::new(FunctionId::new(1), "process")
            .with_params(vec![FfiParam::new(
                "obj",
                FfiDataType::unresolved_handle("MyClass", false),
            )])
            .with_return_type(FfiDataType::resolved(DataType::simple(VOID_TYPE)));

        let lookup = |name: &str| -> Option<TypeId> {
            if name == "MyClass" {
                Some(my_class_id)
            } else {
                None
            }
        };
        let mut instantiate = |_: TypeId, _: Vec<DataType>| -> Result<TypeId, String> {
            Err("no templates".to_string())
        };

        let resolved = func.resolve(&lookup, &mut instantiate).unwrap();

        assert_eq!(resolved.params[0].data_type.type_id, my_class_id);
        assert!(resolved.params[0].data_type.is_handle);
    }

    #[test]
    fn ffi_function_def_resolve_error_unknown_type() {
        let func = FfiFunctionDef::new(FunctionId::new(1), "process")
            .with_params(vec![FfiParam::new(
                "obj",
                FfiDataType::unresolved_simple("UnknownType"),
            )]);

        let lookup = |_: &str| -> Option<TypeId> { None };
        let mut instantiate = |_: TypeId, _: Vec<DataType>| -> Result<TypeId, String> {
            Err("no templates".to_string())
        };

        let result = func.resolve(&lookup, &mut instantiate);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, FfiResolutionError::Parameter { .. }));

        let err_str = format!("{}", err);
        assert!(err_str.contains("obj"));
        assert!(err_str.contains("UnknownType"));
    }

    #[test]
    fn ffi_function_def_resolve_return_type_error() {
        let func = FfiFunctionDef::new(FunctionId::new(1), "create")
            .with_return_type(FfiDataType::unresolved_simple("UnknownType"));

        let lookup = |_: &str| -> Option<TypeId> { None };
        let mut instantiate = |_: TypeId, _: Vec<DataType>| -> Result<TypeId, String> {
            Err("no templates".to_string())
        };

        let result = func.resolve(&lookup, &mut instantiate);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, FfiResolutionError::ReturnType { .. }));
    }

    #[test]
    fn resolved_ffi_function_def_param_types() {
        let resolved = ResolvedFfiFunctionDef {
            id: FunctionId::new(1),
            name: "add".to_string(),
            namespace: Vec::new(),
            params: vec![
                ResolvedFfiParam {
                    name: "a".to_string(),
                    data_type: DataType::simple(INT32_TYPE),
                    default_value: None,
                },
                ResolvedFfiParam {
                    name: "b".to_string(),
                    data_type: DataType::simple(INT32_TYPE),
                    default_value: None,
                },
            ],
            return_type: DataType::simple(INT32_TYPE),
            traits: FunctionTraits::new(),
            owner_type: None,
            operator: None,
            visibility: Visibility::Public,
        };

        let param_types = resolved.param_types();
        assert_eq!(param_types.len(), 2);
        assert_eq!(param_types[0].type_id, INT32_TYPE);
        assert_eq!(param_types[1].type_id, INT32_TYPE);
    }

    #[test]
    fn resolved_ffi_function_def_qualified_name() {
        let resolved = ResolvedFfiFunctionDef {
            id: FunctionId::new(1),
            name: "test".to_string(),
            namespace: vec!["Game".to_string()],
            params: Vec::new(),
            return_type: DataType::simple(VOID_TYPE),
            traits: FunctionTraits::new(),
            owner_type: None,
            operator: None,
            visibility: Visibility::Public,
        };

        assert_eq!(resolved.qualified_name(), "Game::test");
    }

    #[test]
    fn ffi_resolution_error_display() {
        let err = FfiResolutionError::Parameter {
            function: "process".to_string(),
            param_name: "obj".to_string(),
            param_index: 0,
            error: "Unknown type: MyClass".to_string(),
        };

        let msg = format!("{}", err);
        assert!(msg.contains("process"));
        assert!(msg.contains("obj"));
        assert!(msg.contains("0"));
        assert!(msg.contains("MyClass"));
    }

    #[test]
    fn ffi_param_clone() {
        let param = FfiParam::with_default(
            "x",
            FfiDataType::resolved(DataType::simple(INT32_TYPE)),
            FfiExpr::int(42),
        );

        let cloned = param.clone();
        assert_eq!(param, cloned);
    }

    #[test]
    fn resolved_ffi_param_clone() {
        let param = ResolvedFfiParam {
            name: "x".to_string(),
            data_type: DataType::simple(INT32_TYPE),
            default_value: Some(FfiExpr::int(42)),
        };

        let cloned = param.clone();
        assert_eq!(param, cloned);
    }
}
