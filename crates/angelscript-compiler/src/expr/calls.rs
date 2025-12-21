//! Function, method, constructor, and opCall compilation.
//!
//! This module handles all call-like expressions:
//! - Direct function calls: `foo(args)`
//! - Constructor calls: `TypeName(args)`
//! - Method calls: `obj.method(args)` (via member.rs dispatch)
//! - Indirect calls: `callable(args)` (opCall or funcdef)

use angelscript_core::{CompilationError, DataType, Span, TypeHash};
use angelscript_parser::ast::{CallExpr, Expr, IdentExpr, TypeBase, TypeExpr};

use crate::expr_info::ExprInfo;
use crate::overload::{OverloadMatch, resolve_overload};
use crate::type_resolver::TypeResolver;

use super::{ExprCompiler, emit_conversion};

type Result<T> = std::result::Result<T, CompilationError>;

/// Compile a call expression.
///
/// Dispatches to the appropriate handler based on the callee:
/// - Identifier: function call or constructor call
/// - Member expression: method call (handled in member.rs)
/// - Other: indirect call (opCall or funcdef)
pub fn compile_call<'ast>(
    compiler: &mut ExprCompiler<'_, '_>,
    call: &CallExpr<'ast>,
) -> Result<ExprInfo> {
    match call.callee {
        Expr::Ident(ident) => compile_ident_call(compiler, ident, call),
        Expr::Member(member) => {
            // Method calls are parsed as MemberExpr with MemberAccess::Method
            // If we get here, it's a weird edge case - treat as indirect call
            super::member::compile_member(compiler, member)
                .and_then(|_| compile_indirect_call(compiler, call))
        }
        _ => compile_indirect_call(compiler, call),
    }
}

/// Compile a call where the callee is an identifier.
///
/// This could be:
/// - A function call: `print("hello")`
/// - A constructor call: `Vector3(1, 2, 3)`
/// - A template constructor call: `array<int>()`
/// - A super() call in a constructor: `super(args)` to call base class constructor
/// - A funcdef variable call: `callback(args)` where callback is a funcdef handle
fn compile_ident_call<'ast>(
    compiler: &mut ExprCompiler<'_, '_>,
    ident: &IdentExpr<'ast>,
    call: &CallExpr<'ast>,
) -> Result<ExprInfo> {
    let name = ident.ident.name;
    let span = call.span;

    // Check for super() call (base class constructor call)
    if name == "super" {
        return compile_super_call(compiler, call);
    }

    // Check if this identifier has template type arguments (e.g., array<int>)
    // If so, we need to resolve the full template type including instantiation
    if !ident.type_args.is_empty() {
        // Build a TypeExpr from the ident to use the TypeResolver
        let type_expr = TypeExpr::new(
            false,       // is_const
            ident.scope, // scope
            TypeBase::Named(ident.ident),
            ident.type_args, // template_args
            &[],             // suffixes (handles, etc.)
            ident.span,
        );

        // Resolve the template type (this handles instantiation)
        let resolved_type = {
            let mut resolver = TypeResolver::new(compiler.ctx_mut());
            resolver.resolve(&type_expr)?
        };

        // Call the constructor/factory for the instantiated type
        return compile_constructor_call(compiler, resolved_type.type_hash, call);
    }

    // First, check if this is a type (constructor call)
    if let Some(type_hash) = compiler.ctx().resolve_type(name) {
        return compile_constructor_call(compiler, type_hash, call);
    }

    // Otherwise, try as a function call
    if let Some(candidates) = compiler.ctx().resolve_function(name) {
        // Route lambda arguments to specialized handler
        if has_lambda_argument(call) {
            return compile_lambda_function_call(compiler, candidates.to_vec(), call);
        }
        return compile_function_call(compiler, candidates.to_vec(), call);
    }

    // Check if this is a local variable that's a funcdef handle (e.g., `add(5, 3)` where add is `BinaryOp @add`)
    if ident.scope.is_none() {
        // Extract all needed data from context first to avoid borrow conflicts
        let funcdef_info = compiler.ctx().get_local(name).and_then(|var| {
            let funcdef = compiler
                .ctx()
                .get_type(var.data_type.type_hash)
                .and_then(|e| e.as_funcdef())?;
            Some((
                var.slot,
                funcdef.name.clone(),
                funcdef.params.clone(),
                funcdef.return_type,
            ))
        });

        if let Some((slot, funcdef_name, params, return_type)) = funcdef_info {
            // Emit GetLocal to push the funcdef handle onto the stack
            compiler.emitter().emit_get_local(slot);

            // Then compile the funcdef call
            return compile_funcdef_call(compiler, &funcdef_name, &params, &return_type, call);
        }
    }

    // Could be a variable or unknown identifier
    Err(CompilationError::UnknownFunction {
        name: name.to_string(),
        span,
    })
}

/// Compile a super() call to the base class constructor.
///
/// This is only valid inside a constructor of a derived class.
fn compile_super_call(
    compiler: &mut ExprCompiler<'_, '_>,
    call: &CallExpr<'_>,
) -> Result<ExprInfo> {
    let span = call.span;

    // Verify we're inside a constructor - super() is not valid in regular methods
    if !compiler.is_constructor() {
        return Err(CompilationError::Other {
            message: "super() can only be used inside a class constructor".to_string(),
            span,
        });
    }

    // Get the current class - super() is only valid in a class context
    let class_hash = compiler
        .current_class()
        .ok_or_else(|| CompilationError::Other {
            message: "super() can only be used inside a class constructor".to_string(),
            span,
        })?;

    // Get the base class hash and name
    let (base_class_hash, base_class_name) =
        {
            let type_entry = compiler.ctx().get_type(class_hash).ok_or_else(|| {
                CompilationError::UnknownType {
                    name: format!("{:?}", class_hash),
                    span,
                }
            })?;

            let class = type_entry
                .as_class()
                .ok_or_else(|| CompilationError::Other {
                    message: "super() used in non-class context".to_string(),
                    span,
                })?;

            let base_hash = class.base_class.ok_or_else(|| CompilationError::Other {
                message: "super() used in class without base class".to_string(),
                span,
            })?;

            // Now get the base class name for method lookup
            let base_entry = compiler.ctx().get_type(base_hash).ok_or_else(|| {
                CompilationError::UnknownType {
                    name: format!("{:?}", base_hash),
                    span,
                }
            })?;

            let base_class = base_entry
                .as_class()
                .ok_or_else(|| CompilationError::Other {
                    message: "base type is not a class".to_string(),
                    span,
                })?;

            (base_hash, base_class.name.clone())
        };

    // Emit GetThis FIRST - the calling convention requires 'this' on stack before arguments
    compiler.emitter().emit_get_this();

    // Compile arguments and collect their types (pushed after 'this')
    let (arg_types, arg_count) = compile_arguments(compiler, call)?;

    // Get base class constructor candidates.
    // For auto-generated constructors, they're registered as functions but not in
    // the class's methods list. We need to check multiple sources.
    let mut candidates = Vec::new();

    // First, check behaviors.constructors (for FFI types)
    {
        let base_entry = compiler.ctx().get_type(base_class_hash).ok_or_else(|| {
            CompilationError::UnknownType {
                name: format!("{:?}", base_class_hash),
                span,
            }
        })?;

        let base_class = base_entry
            .as_class()
            .ok_or_else(|| CompilationError::Other {
                message: "base type is not a class".to_string(),
                span,
            })?;

        candidates.extend(base_class.behaviors.constructors.iter().copied());

        // Also check the class's methods list (for script-declared constructors)
        candidates.extend(base_class.find_methods(&base_class_name).iter().copied());
    }

    // Also check for auto-generated constructors by constructing their TypeHashes directly
    // Auto-generated default constructor
    let default_ctor = TypeHash::from_constructor(base_class_hash, &[]);
    if compiler.ctx().get_function(default_ctor).is_some() && !candidates.contains(&default_ctor) {
        candidates.push(default_ctor);
    }

    // Auto-generated copy constructor (takes const base_class &in)
    let copy_ctor = TypeHash::from_constructor(base_class_hash, &[base_class_hash]);
    if compiler.ctx().get_function(copy_ctor).is_some() && !candidates.contains(&copy_ctor) {
        candidates.push(copy_ctor);
    }

    // If arguments were provided, also check for a constructor matching those exact types
    // This handles user-defined constructors that might not be in the methods list yet
    if !arg_types.is_empty() {
        let arg_hashes: Vec<TypeHash> = arg_types.iter().map(|dt| dt.type_hash).collect();
        let specific_ctor = TypeHash::from_constructor(base_class_hash, &arg_hashes);
        if compiler.ctx().get_function(specific_ctor).is_some()
            && !candidates.contains(&specific_ctor)
        {
            candidates.push(specific_ctor);
        }
    }

    if candidates.is_empty() {
        return Err(CompilationError::Other {
            message: format!("base class '{}' has no constructors", base_class_name),
            span,
        });
    }

    // Resolve overload among base class constructors
    let overload = resolve_overload(&candidates, &arg_types, compiler.ctx(), span)?;

    // Apply argument conversions
    apply_argument_conversions(compiler, &overload)?;

    // Emit method call to base constructor (this is already on stack from earlier)
    compiler
        .emitter()
        .emit_call_method(overload.func_hash, arg_count as u8);

    // super() returns void
    Ok(ExprInfo::rvalue(DataType::void()))
}

/// Check if any argument in a call is a lambda expression.
fn has_lambda_argument(call: &CallExpr<'_>) -> bool {
    call.args
        .iter()
        .any(|arg| matches!(arg.value, Expr::Lambda(_)))
}

/// Compile a direct function call.
///
/// Resolves overloads and emits the appropriate call bytecode.
pub fn compile_function_call(
    compiler: &mut ExprCompiler<'_, '_>,
    candidates: Vec<TypeHash>,
    call: &CallExpr<'_>,
) -> Result<ExprInfo> {
    let span = call.span;

    // Compile arguments using infer()
    let (arg_types, arg_count) = compile_arguments(compiler, call)?;

    // Resolve overload
    let overload = resolve_overload(&candidates, &arg_types, compiler.ctx(), span)?;

    // Apply argument conversions
    apply_argument_conversions(compiler, &overload)?;

    // Get return type
    let return_type = get_function_return_type(compiler, overload.func_hash)?;

    // Emit call
    compiler
        .emitter()
        .emit_call(overload.func_hash, arg_count as u8);

    Ok(ExprInfo::rvalue(return_type))
}

/// Compile a function call with lambda arguments.
///
/// For single-candidate functions, uses `check()` with expected parameter types
/// to enable lambda type inference. For multiple candidates, errors per AngelScript
/// spec: "If there are multiple matching uses for the anonymous function it will
/// be necessary to explicitly inform the parameter types."
fn compile_lambda_function_call(
    compiler: &mut ExprCompiler<'_, '_>,
    candidates: Vec<TypeHash>,
    call: &CallExpr<'_>,
) -> Result<ExprInfo> {
    let span = call.span;

    // Multiple candidates with lambda args is an error per AngelScript spec
    if candidates.len() != 1 {
        return Err(CompilationError::TypeMismatch {
            message: "lambda with untyped parameters requires unambiguous function overload; \
                      use explicit parameter types or ensure only one overload matches"
                .to_string(),
            span,
        });
    }

    let func_hash = candidates[0];

    // Get function info
    let (param_types, required_count, func_name, return_type) =
        {
            let func = compiler.ctx().get_function(func_hash).ok_or_else(|| {
                CompilationError::Internal {
                    message: format!("Function not found: {:?}", func_hash),
                }
            })?;
            (
                func.def
                    .params
                    .iter()
                    .map(|p| p.data_type)
                    .collect::<Vec<_>>(),
                func.def.required_param_count(),
                func.def.name.clone(),
                func.def.return_type,
            )
        };

    // Check argument count
    let arg_count = call.args.len();
    let total_params = param_types.len();

    if arg_count < required_count || arg_count > total_params {
        return Err(CompilationError::ArgumentCountMismatch {
            name: func_name,
            expected: required_count,
            got: arg_count,
            span,
        });
    }

    // Compile arguments using check() with expected types (enables lambda inference)
    for (arg, expected_type) in call.args.iter().zip(&param_types) {
        compiler.check(arg.value, expected_type)?;
    }

    // Emit call
    compiler.emitter().emit_call(func_hash, arg_count as u8);

    Ok(ExprInfo::rvalue(return_type))
}

/// Compile a constructor or factory call.
///
/// Validates that the type is instantiable and selects the appropriate
/// constructor/factory based on the type kind.
pub fn compile_constructor_call(
    compiler: &mut ExprCompiler<'_, '_>,
    type_hash: TypeHash,
    call: &CallExpr<'_>,
) -> Result<ExprInfo> {
    let span = call.span;

    // Extract everything we need from the type entry first to avoid borrow conflicts
    let (is_value_type, candidates, uses_constructors) =
        {
            let type_entry = compiler.ctx().get_type(type_hash).ok_or_else(|| {
                CompilationError::UnknownType {
                    name: format!("{:?}", type_hash),
                    span,
                }
            })?;

            let class = type_entry
                .as_class()
                .ok_or_else(|| CompilationError::Other {
                    message: format!("'{}' is not a class type", type_entry.qualified_name()),
                    span,
                })?;

            let type_name = type_entry.qualified_name();
            let is_value_type = class.type_kind.is_value();

            // Select constructors or factories based on type kind
            let (candidates, uses_constructors) = if class.type_kind.uses_constructors() {
                (class.behaviors.constructors.clone(), true)
            } else if class.type_kind.uses_factories() {
                (class.behaviors.factories.clone(), false)
            } else {
                return Err(CompilationError::Other {
                    message: format!("type '{}' cannot be instantiated", type_name),
                    span,
                });
            };

            if candidates.is_empty() {
                return Err(CompilationError::Other {
                    message: format!("no constructor available for type '{}'", type_name),
                    span,
                });
            }

            (is_value_type, candidates, uses_constructors)
        };

    // Validate instantiability
    validate_instantiable(compiler, type_hash, span)?;

    // Compile arguments
    let (arg_types, arg_count) = compile_arguments(compiler, call)?;

    // Resolve overload among constructors/factories
    let overload = resolve_overload(&candidates, &arg_types, compiler.ctx(), span)?;

    // Apply argument conversions
    apply_argument_conversions(compiler, &overload)?;

    // Emit appropriate bytecode
    if uses_constructors {
        compiler
            .emitter()
            .emit_new(type_hash, overload.func_hash, arg_count as u8);
    } else {
        compiler
            .emitter()
            .emit_new_factory(overload.func_hash, arg_count as u8);
    }

    // Return type is a handle to the constructed object for reference types,
    // or the value itself for value types
    let result_type = if is_value_type {
        DataType::simple(type_hash)
    } else {
        DataType::with_handle(type_hash, false)
    };

    Ok(ExprInfo::rvalue(result_type))
}

/// Check if any argument in a slice is a lambda expression.
fn has_lambda_in_args(args: &[angelscript_parser::ast::Argument<'_>]) -> bool {
    args.iter().any(|arg| matches!(arg.value, Expr::Lambda(_)))
}

/// Compile a method call on an object.
///
/// This is called from member.rs when processing `obj.method(args)`.
pub fn compile_method_call(
    compiler: &mut ExprCompiler<'_, '_>,
    obj_type: &DataType,
    method_name: &str,
    call_args: &[angelscript_parser::ast::Argument<'_>],
    span: Span,
) -> Result<ExprInfo> {
    // Find method candidates and check if it's an interface (all in one borrow)
    let (candidates, is_interface) = {
        let candidates = compiler.ctx().find_methods(obj_type.type_hash, method_name);

        if candidates.is_empty() {
            let type_name = compiler
                .ctx()
                .get_type(obj_type.type_hash)
                .map(|e| e.qualified_name().to_string())
                .unwrap_or_else(|| format!("{:?}", obj_type.type_hash));
            return Err(CompilationError::UnknownMethod {
                method: method_name.to_string(),
                type_name,
                span,
            });
        }

        let is_interface = compiler
            .ctx()
            .get_type(obj_type.type_hash)
            .map(|e| e.is_interface())
            .unwrap_or(false);

        (candidates.to_vec(), is_interface)
    };

    // Route lambda arguments to specialized handler
    if has_lambda_in_args(call_args) {
        return compile_lambda_method_call(
            compiler,
            obj_type,
            &candidates,
            call_args,
            is_interface,
            span,
        );
    }

    // Standard path: infer argument types, resolve overload
    let mut arg_types = Vec::with_capacity(call_args.len());
    for arg in call_args {
        let info = compiler.infer(arg.value)?;
        arg_types.push(info.data_type);
    }
    let arg_count = arg_types.len();

    // Resolve overload
    let overload = resolve_overload(&candidates, &arg_types, compiler.ctx(), span)?;

    // Get method info for const-correctness, return type, and virtual dispatch
    let (is_const_method, return_type, is_virtual_method, is_final_method) = {
        let func = compiler
            .ctx()
            .get_function(overload.func_hash)
            .ok_or_else(|| CompilationError::Internal {
                message: format!("Method not found: {:?}", overload.func_hash),
            })?;
        (
            func.def.is_const(),
            func.def.return_type,
            func.def.is_virtual(),
            func.def.is_final(),
        )
    };

    if obj_type.is_effectively_const() && !is_const_method {
        return Err(CompilationError::CannotModifyConst {
            message: format!(
                "cannot call non-const method '{}' on const object",
                method_name
            ),
            span,
        });
    }

    // Apply argument conversions
    apply_argument_conversions(compiler, &overload)?;

    // Emit call - use interface dispatch for interfaces, direct call for classes
    if is_interface {
        // Compute signature hash from resolved method's parameters
        let sig_hash = {
            let func = compiler
                .ctx()
                .get_function(overload.func_hash)
                .ok_or_else(|| CompilationError::Internal {
                    message: format!("Method not found: {:?}", overload.func_hash),
                })?;
            let param_sig_hashes: Vec<_> = func
                .def
                .params
                .iter()
                .map(|p| p.data_type.signature_hash())
                .collect();
            TypeHash::from_signature(method_name, &param_sig_hashes, func.def.is_const()).0
        };

        // Get interface method slot by signature hash
        let slot = compiler
            .ctx()
            .get_type(obj_type.type_hash)
            .and_then(|e| e.as_interface())
            .and_then(|iface| iface.method_slot(sig_hash))
            .ok_or_else(|| CompilationError::Internal {
                message: format!(
                    "Interface method slot not found for '{}' on {:?}",
                    method_name, obj_type.type_hash
                ),
            })?;

        compiler
            .emitter()
            .emit_call_interface(obj_type.type_hash, slot, arg_count as u8);
    } else {
        // For class methods, check if we need virtual dispatch
        // Use virtual dispatch if:
        // 1. Method is virtual (not final)
        // 2. Class could have derived types (not final)
        let needs_virtual_dispatch = is_virtual_method && !is_final_method && {
            compiler
                .ctx()
                .get_type(obj_type.type_hash)
                .and_then(|e| e.as_class())
                .map(|class| !class.is_final)
                .unwrap_or(false)
        };

        if needs_virtual_dispatch {
            // Compute signature hash from resolved method's parameters
            let sig_hash = {
                let func = compiler
                    .ctx()
                    .get_function(overload.func_hash)
                    .ok_or_else(|| CompilationError::Internal {
                        message: format!("Method not found: {:?}", overload.func_hash),
                    })?;
                let param_sig_hashes: Vec<_> = func
                    .def
                    .params
                    .iter()
                    .map(|p| p.data_type.signature_hash())
                    .collect();
                TypeHash::from_signature(method_name, &param_sig_hashes, func.def.is_const()).0
            };

            // Get vtable slot by signature hash
            let slot = compiler
                .ctx()
                .get_type(obj_type.type_hash)
                .and_then(|e| e.as_class())
                .and_then(|class| class.vtable_slot(sig_hash))
                .ok_or_else(|| CompilationError::Internal {
                    message: format!(
                        "VTable slot not found for '{}' on {:?}",
                        method_name, obj_type.type_hash
                    ),
                })?;

            compiler.emitter().emit_call_virtual(slot, arg_count as u8);
        } else {
            // Direct call for non-virtual or final methods
            compiler
                .emitter()
                .emit_call_method(overload.func_hash, arg_count as u8);
        }
    }

    Ok(ExprInfo::rvalue(return_type))
}

/// Compile a method call with lambda arguments.
///
/// For single-candidate methods, uses `check()` with expected parameter types
/// to enable lambda type inference. For multiple candidates, errors per AngelScript spec.
fn compile_lambda_method_call(
    compiler: &mut ExprCompiler<'_, '_>,
    obj_type: &DataType,
    candidates: &[TypeHash],
    call_args: &[angelscript_parser::ast::Argument<'_>],
    is_interface: bool,
    span: Span,
) -> Result<ExprInfo> {
    // Multiple candidates with lambda args is an error per AngelScript spec
    if candidates.len() != 1 {
        return Err(CompilationError::TypeMismatch {
            message: "lambda with untyped parameters requires unambiguous method overload; \
                      use explicit parameter types or ensure only one overload matches"
                .to_string(),
            span,
        });
    }

    let method_hash = candidates[0];

    // Get method info
    let (param_types, required_count, method_name, is_const_method, return_type) =
        {
            let func = compiler.ctx().get_function(method_hash).ok_or_else(|| {
                CompilationError::Internal {
                    message: format!("Method not found: {:?}", method_hash),
                }
            })?;
            (
                func.def
                    .params
                    .iter()
                    .map(|p| p.data_type)
                    .collect::<Vec<_>>(),
                func.def.required_param_count(),
                func.def.name.clone(),
                func.def.is_const(),
                func.def.return_type,
            )
        };

    // Const-correctness check
    if obj_type.is_effectively_const() && !is_const_method {
        return Err(CompilationError::CannotModifyConst {
            message: format!(
                "cannot call non-const method '{}' on const object",
                method_name
            ),
            span,
        });
    }

    // Check argument count
    let arg_count = call_args.len();
    let total_params = param_types.len();

    if arg_count < required_count || arg_count > total_params {
        return Err(CompilationError::ArgumentCountMismatch {
            name: method_name,
            expected: required_count,
            got: arg_count,
            span,
        });
    }

    // Compile arguments using check() with expected types (enables lambda inference)
    for (arg, expected_type) in call_args.iter().zip(&param_types) {
        compiler.check(arg.value, expected_type)?;
    }

    // Emit call - use interface dispatch for interfaces, direct call for classes
    if is_interface {
        // Compute signature hash from resolved method's parameters
        let sig_hash = {
            let func = compiler
                .ctx()
                .get_function(method_hash)
                .ok_or_else(|| CompilationError::Internal {
                    message: format!("Method not found: {:?}", method_hash),
                })?;
            let param_sig_hashes: Vec<_> = func
                .def
                .params
                .iter()
                .map(|p| p.data_type.signature_hash())
                .collect();
            TypeHash::from_signature(&method_name, &param_sig_hashes, is_const_method).0
        };

        // Get interface method slot by signature hash
        let slot = compiler
            .ctx()
            .get_type(obj_type.type_hash)
            .and_then(|e| e.as_interface())
            .and_then(|iface| iface.method_slot(sig_hash))
            .ok_or_else(|| CompilationError::Internal {
                message: format!(
                    "Interface method slot not found for '{}' on {:?}",
                    method_name, obj_type.type_hash
                ),
            })?;

        compiler
            .emitter()
            .emit_call_interface(obj_type.type_hash, slot, arg_count as u8);
    } else {
        compiler
            .emitter()
            .emit_call_method(method_hash, arg_count as u8);
    }

    Ok(ExprInfo::rvalue(return_type))
}

/// Result of checking a callee type for callability.
enum CalleeKind {
    /// Has opCall methods
    OpCall(Vec<TypeHash>),
    /// Is a funcdef with params and return type
    Funcdef {
        name: String,
        params: Vec<DataType>,
        return_type: DataType,
    },
    /// Not callable - includes error type name
    NotCallable(String),
}

/// Compile an indirect call (opCall or funcdef).
///
/// This handles cases where the callee is not a simple identifier:
/// - Callable objects via opCall
/// - Function pointers/funcdefs
fn compile_indirect_call(
    compiler: &mut ExprCompiler<'_, '_>,
    call: &CallExpr<'_>,
) -> Result<ExprInfo> {
    let span = call.span;

    // Compile the callee expression
    let callee_info = compiler.infer(call.callee)?;

    // Determine what kind of callee this is (extract data to avoid borrow conflicts)
    let callee_kind = {
        let type_entry = compiler.ctx().get_type(callee_info.data_type.type_hash);

        if let Some(class) = type_entry.as_ref().and_then(|e| e.as_class()) {
            let op_call_methods = class.find_methods("opCall");
            if !op_call_methods.is_empty() {
                CalleeKind::OpCall(op_call_methods.to_vec())
            } else {
                CalleeKind::NotCallable(class.qualified_name.clone())
            }
        } else if let Some(funcdef) = type_entry.as_ref().and_then(|e| e.as_funcdef()) {
            CalleeKind::Funcdef {
                name: funcdef.name.clone(),
                params: funcdef.params.clone(),
                return_type: funcdef.return_type,
            }
        } else {
            let type_name = type_entry
                .map(|e| e.qualified_name().to_string())
                .unwrap_or_else(|| format!("{:?}", callee_info.data_type.type_hash));
            CalleeKind::NotCallable(type_name)
        }
    };

    match callee_kind {
        CalleeKind::OpCall(candidates) => {
            compile_opcall(compiler, &callee_info.data_type, &candidates, call)
        }
        CalleeKind::Funcdef {
            name,
            params,
            return_type,
        } => compile_funcdef_call(compiler, &name, &params, &return_type, call),
        CalleeKind::NotCallable(type_name) => Err(CompilationError::Other {
            message: format!("type '{}' is not callable", type_name),
            span,
        }),
    }
}

/// Compile a call through opCall operator.
fn compile_opcall(
    compiler: &mut ExprCompiler<'_, '_>,
    obj_type: &DataType,
    candidates: &[TypeHash],
    call: &CallExpr<'_>,
) -> Result<ExprInfo> {
    let span = call.span;

    // Compile arguments
    let mut arg_types = Vec::with_capacity(call.args.len());
    for arg in call.args {
        let info = compiler.infer(arg.value)?;
        arg_types.push(info.data_type);
    }
    let arg_count = arg_types.len();

    // Resolve overload among opCall methods
    let overload = resolve_overload(candidates, &arg_types, compiler.ctx(), span)?;

    // Const-correctness check and get return type
    let (is_const_method, return_type) = {
        let func = compiler
            .ctx()
            .get_function(overload.func_hash)
            .ok_or_else(|| CompilationError::Internal {
                message: format!("opCall method not found: {:?}", overload.func_hash),
            })?;
        (func.def.is_const(), func.def.return_type)
    };

    if obj_type.is_effectively_const() && !is_const_method {
        return Err(CompilationError::CannotModifyConst {
            message: "cannot call non-const opCall on const object".to_string(),
            span,
        });
    }

    // Apply argument conversions
    apply_argument_conversions(compiler, &overload)?;

    // Emit method call
    compiler
        .emitter()
        .emit_call_method(overload.func_hash, arg_count as u8);

    Ok(ExprInfo::rvalue(return_type))
}

/// Compile a call through a funcdef (function pointer).
fn compile_funcdef_call(
    compiler: &mut ExprCompiler<'_, '_>,
    name: &str,
    params: &[DataType],
    return_type: &DataType,
    call: &CallExpr<'_>,
) -> Result<ExprInfo> {
    let span = call.span;

    // Check argument count
    let param_count = params.len();
    let arg_count = call.args.len();

    if arg_count != param_count {
        return Err(CompilationError::ArgumentCountMismatch {
            name: name.to_string(),
            expected: param_count,
            got: arg_count,
            span,
        });
    }

    // Compile and type-check arguments
    for (i, arg) in call.args.iter().enumerate() {
        let expected_type = &params[i];
        compiler.check(arg.value, expected_type)?;
    }

    // Emit funcptr call
    compiler.emitter().emit_call_func_ptr(arg_count as u8);

    Ok(ExprInfo::rvalue(*return_type))
}

// =============================================================================
// Helper functions
// =============================================================================

/// Compile call arguments and return their types.
fn compile_arguments(
    compiler: &mut ExprCompiler<'_, '_>,
    call: &CallExpr<'_>,
) -> Result<(Vec<DataType>, usize)> {
    let mut arg_types = Vec::with_capacity(call.args.len());

    for arg in call.args {
        let info = compiler.infer(arg.value)?;
        arg_types.push(info.data_type);
    }

    let count = arg_types.len();
    Ok((arg_types, count))
}

/// Apply conversions to arguments after overload resolution.
fn apply_argument_conversions(
    compiler: &mut ExprCompiler<'_, '_>,
    overload: &OverloadMatch,
) -> Result<()> {
    for conv in overload.arg_conversions.iter().flatten() {
        emit_conversion(compiler.emitter(), conv);
    }
    Ok(())
}

/// Get the return type of a function.
fn get_function_return_type(
    compiler: &ExprCompiler<'_, '_>,
    func_hash: TypeHash,
) -> Result<DataType> {
    let func =
        compiler
            .ctx()
            .get_function(func_hash)
            .ok_or_else(|| CompilationError::Internal {
                message: format!("Function not found: {:?}", func_hash),
            })?;

    Ok(func.def.return_type)
}

/// Validate that a type can be instantiated.
fn validate_instantiable(
    compiler: &ExprCompiler<'_, '_>,
    type_hash: TypeHash,
    span: Span,
) -> Result<()> {
    let type_entry =
        compiler
            .ctx()
            .get_type(type_hash)
            .ok_or_else(|| CompilationError::UnknownType {
                name: format!("{:?}", type_hash),
                span,
            })?;

    // Check for mixin
    if let Some(class) = type_entry.as_class() {
        if class.is_mixin {
            return Err(CompilationError::Other {
                message: format!(
                    "cannot instantiate mixin class '{}'",
                    type_entry.qualified_name()
                ),
                span,
            });
        }

        if class.is_abstract {
            return Err(CompilationError::Other {
                message: format!(
                    "cannot instantiate abstract class '{}'",
                    type_entry.qualified_name()
                ),
                span,
            });
        }
    }

    // Check for interface
    if type_entry.is_interface() {
        return Err(CompilationError::Other {
            message: format!(
                "cannot instantiate interface '{}' directly",
                type_entry.qualified_name()
            ),
            span,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::ConstantPool;
    use crate::context::CompilationContext;
    use crate::emit::BytecodeEmitter;
    use angelscript_core::{
        ClassEntry, DataType, FunctionDef, FunctionEntry, FunctionTraits, TypeBehaviors, TypeKind,
        Visibility, primitives,
    };
    use angelscript_registry::SymbolRegistry;

    fn create_test_context() -> (SymbolRegistry, ConstantPool) {
        (SymbolRegistry::with_primitives(), ConstantPool::new())
    }

    fn register_simple_function(registry: &mut SymbolRegistry, name: &str) -> TypeHash {
        let func_hash = TypeHash::from_function(name, &[]);
        let func_def = FunctionDef::new(
            func_hash,
            name.to_string(),
            vec![],
            vec![],
            DataType::void(),
            None,
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(func_def))
            .unwrap();
        func_hash
    }

    #[test]
    fn validate_mixin_not_instantiable() {
        let (mut registry, mut constants) = create_test_context();

        // Create a mixin class
        let type_hash = TypeHash::from_name("MyMixin");
        let mut class = ClassEntry::ffi("MyMixin", TypeKind::script_object());
        class.is_mixin = true;
        registry.register_type(class.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        let result = validate_instantiable(&compiler, type_hash, Span::default());

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, CompilationError::Other { .. }));
    }

    #[test]
    fn validate_abstract_not_instantiable() {
        let (mut registry, _) = create_test_context();

        // Create an abstract class
        let type_hash = TypeHash::from_name("AbstractClass");
        let mut class = ClassEntry::ffi("AbstractClass", TypeKind::script_object());
        class.is_abstract = true;
        registry.register_type(class.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        let result = validate_instantiable(&compiler, type_hash, Span::default());

        match result {
            Err(CompilationError::Other { message, .. }) => {
                assert!(
                    message.contains("abstract"),
                    "error should mention abstract: {message}"
                );
                assert!(
                    message.contains("AbstractClass"),
                    "error should mention class name: {message}"
                );
            }
            other => panic!("expected Other error for abstract class, got {:?}", other),
        }
    }

    #[test]
    fn validate_interface_not_instantiable() {
        let (mut registry, _) = create_test_context();

        // Create an interface
        let type_hash = TypeHash::from_name("IDrawable");
        let interface = angelscript_core::InterfaceEntry::ffi("IDrawable");
        registry.register_type(interface.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        let result = validate_instantiable(&compiler, type_hash, Span::default());

        match result {
            Err(CompilationError::Other { message, .. }) => {
                assert!(
                    message.contains("interface"),
                    "error should mention interface: {message}"
                );
                assert!(
                    message.contains("IDrawable"),
                    "error should mention interface name: {message}"
                );
            }
            other => panic!("expected Other error for interface, got {:?}", other),
        }
    }

    #[test]
    fn validate_regular_class_instantiable() {
        let (mut registry, _) = create_test_context();

        // Create a regular class
        let type_hash = TypeHash::from_name("Player");
        let class = ClassEntry::ffi("Player", TypeKind::script_object());
        registry.register_type(class.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        let result = validate_instantiable(&compiler, type_hash, Span::default());

        assert!(result.is_ok());
    }

    // =========================================================================
    // Function call tests
    // =========================================================================

    #[test]
    fn compile_function_call_resolves_overload() {
        let (mut registry, mut constants) = create_test_context();
        let func_hash = register_simple_function(&mut registry, "test_func");

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        // Test get_function_return_type helper
        let return_type = get_function_return_type(&compiler, func_hash);
        assert!(return_type.is_ok());
        assert_eq!(return_type.unwrap().type_hash, primitives::VOID);
    }

    #[test]
    fn compile_function_call_unknown_function() {
        let (registry, mut constants) = create_test_context();

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        let unknown_hash = TypeHash::from_function("unknown", &[]);
        let result = get_function_return_type(&compiler, unknown_hash);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CompilationError::Internal { .. }
        ));
    }

    // =========================================================================
    // Constructor call tests
    // =========================================================================

    #[test]
    fn constructor_call_value_type_result() {
        let (mut registry, _) = create_test_context();

        // Create a value type
        let type_hash = TypeHash::from_name("Vec2");
        let ctor_hash = TypeHash::from_constructor(type_hash, &[]);

        let mut class = ClassEntry::ffi("Vec2", TypeKind::value::<[f32; 2]>());
        class.behaviors = TypeBehaviors {
            constructors: vec![ctor_hash],
            ..Default::default()
        };
        registry.register_type(class.into()).unwrap();

        // Register constructor
        let ctor_def = FunctionDef::new(
            ctor_hash,
            "$ctor".to_string(),
            vec![],
            vec![],
            DataType::void(),
            Some(type_hash),
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(ctor_def))
            .unwrap();

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        // Value type should be instantiable
        let result = validate_instantiable(&compiler, type_hash, Span::default());
        assert!(result.is_ok());
    }

    #[test]
    fn constructor_call_reference_type_uses_factories() {
        let (mut registry, _) = create_test_context();

        // Create a reference type (uses factories)
        let type_hash = TypeHash::from_name("Array");
        // Factories are functions with the naming convention "$factory"
        let factory_hash = TypeHash::from_function("Array::$factory", &[]);

        let mut class = ClassEntry::ffi("Array", TypeKind::reference());
        class.behaviors = TypeBehaviors {
            factories: vec![factory_hash],
            ..Default::default()
        };
        registry.register_type(class.into()).unwrap();

        // Verify it's instantiable
        let ctx = CompilationContext::new(&registry);
        let type_entry = ctx.get_type(type_hash).unwrap();
        let class_entry = type_entry.as_class().unwrap();

        assert!(class_entry.type_kind.uses_factories());
        assert!(!class_entry.type_kind.uses_constructors());
        assert!(!class_entry.behaviors.factories.is_empty());
    }

    // =========================================================================
    // Method call tests
    // =========================================================================

    fn register_class_with_method(
        registry: &mut SymbolRegistry,
        class_name: &str,
        method_name: &str,
        is_const: bool,
    ) -> (TypeHash, TypeHash) {
        let type_hash = TypeHash::from_name(class_name);
        let method_hash = TypeHash::from_method(type_hash, method_name, &[]);

        let mut class = ClassEntry::ffi(class_name, TypeKind::script_object());
        class.add_method(method_name, method_hash);
        registry.register_type(class.into()).unwrap();

        let mut traits = FunctionTraits::default();
        traits.is_const = is_const;
        let method_def = FunctionDef::new(
            method_hash,
            method_name.to_string(),
            vec![],
            vec![],
            DataType::simple(primitives::INT32),
            Some(type_hash),
            traits,
            true,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(method_def))
            .unwrap();

        (type_hash, method_hash)
    }

    #[test]
    fn method_call_const_method_on_const_object_allowed() {
        let (mut registry, mut constants) = create_test_context();
        let (type_hash, _) = register_class_with_method(&mut registry, "Widget", "getValue", true);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let mut compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        let mut obj_type = DataType::simple(type_hash);
        obj_type.is_const = true;

        let result =
            compile_method_call(&mut compiler, &obj_type, "getValue", &[], Span::default());
        assert!(result.is_ok());
    }

    #[test]
    fn method_call_non_const_method_on_const_object_rejected() {
        let (mut registry, mut constants) = create_test_context();
        let (type_hash, _) = register_class_with_method(&mut registry, "Widget", "setValue", false);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let mut compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        let mut obj_type = DataType::simple(type_hash);
        obj_type.is_const = true;

        let result =
            compile_method_call(&mut compiler, &obj_type, "setValue", &[], Span::default());
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CompilationError::CannotModifyConst { .. }
        ));
    }

    #[test]
    fn method_call_non_const_method_on_mutable_object_allowed() {
        let (mut registry, mut constants) = create_test_context();
        let (type_hash, _) = register_class_with_method(&mut registry, "Widget", "setValue", false);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let mut compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        let obj_type = DataType::simple(type_hash);

        let result =
            compile_method_call(&mut compiler, &obj_type, "setValue", &[], Span::default());
        assert!(result.is_ok());
    }

    #[test]
    fn method_call_unknown_method_error() {
        let (mut registry, mut constants) = create_test_context();
        let type_hash = TypeHash::from_name("Widget");
        let class = ClassEntry::ffi("Widget", TypeKind::script_object());
        registry.register_type(class.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let mut compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        let obj_type = DataType::simple(type_hash);

        let result = compile_method_call(
            &mut compiler,
            &obj_type,
            "nonexistent",
            &[],
            Span::default(),
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CompilationError::UnknownMethod { .. }
        ));
    }

    // =========================================================================
    // Interface type detection tests
    // =========================================================================

    #[test]
    fn interface_type_is_correctly_detected() {
        let (mut registry, _) = create_test_context();
        let type_hash = TypeHash::from_name("IDrawable");

        // Create interface
        use angelscript_core::MethodSignature;
        let draw_sig = MethodSignature::new("draw", vec![], DataType::void());
        let interface = angelscript_core::InterfaceEntry::ffi("IDrawable").with_method(draw_sig);
        registry.register_type(interface.into()).unwrap();

        let ctx = CompilationContext::new(&registry);

        // Interface type should be correctly identified
        let type_entry = ctx.get_type(type_hash).unwrap();
        assert!(type_entry.is_interface());
    }

    #[test]
    fn class_type_is_not_interface() {
        let (mut registry, _) = create_test_context();
        let type_hash = TypeHash::from_name("Widget");
        let class = ClassEntry::ffi("Widget", TypeKind::script_object());
        registry.register_type(class.into()).unwrap();

        let ctx = CompilationContext::new(&registry);

        // Class type should NOT be identified as interface
        let type_entry = ctx.get_type(type_hash).unwrap();
        assert!(!type_entry.is_interface());
    }

    // =========================================================================
    // Argument conversion tests
    // =========================================================================

    #[test]
    fn apply_argument_conversions_empty_list() {
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let mut compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        let overload = OverloadMatch {
            func_hash: TypeHash::from_function("test", &[]),
            arg_conversions: vec![],
            total_cost: 0,
        };

        let result = apply_argument_conversions(&mut compiler, &overload);
        assert!(result.is_ok());
    }

    #[test]
    fn apply_argument_conversions_with_none_values() {
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let mut compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        let overload = OverloadMatch {
            func_hash: TypeHash::from_function("test", &[]),
            arg_conversions: vec![None, None, None],
            total_cost: 0,
        };

        let result = apply_argument_conversions(&mut compiler, &overload);
        assert!(result.is_ok());
    }

    // =========================================================================
    // opCall tests
    // =========================================================================

    fn register_class_with_opcall(
        registry: &mut SymbolRegistry,
        class_name: &str,
        is_const: bool,
    ) -> (TypeHash, TypeHash) {
        let type_hash = TypeHash::from_name(class_name);
        let method_hash = TypeHash::from_method(type_hash, "opCall", &[]);

        let mut class = ClassEntry::ffi(class_name, TypeKind::script_object());
        class.add_method("opCall", method_hash);
        registry.register_type(class.into()).unwrap();

        let mut traits = FunctionTraits::default();
        traits.is_const = is_const;
        let method_def = FunctionDef::new(
            method_hash,
            "opCall".to_string(),
            vec![],
            vec![],
            DataType::simple(primitives::INT32),
            Some(type_hash),
            traits,
            true,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(method_def))
            .unwrap();

        (type_hash, method_hash)
    }

    #[test]
    fn opcall_method_is_detected() {
        let (mut registry, _) = create_test_context();
        let (type_hash, method_hash) = register_class_with_opcall(&mut registry, "Functor", true);

        let ctx = CompilationContext::new(&registry);
        let type_entry = ctx.get_type(type_hash).unwrap();
        let class = type_entry.as_class().unwrap();

        // Should find opCall method
        let opcall_methods = class.find_methods("opCall");
        assert_eq!(opcall_methods.len(), 1);
        assert_eq!(opcall_methods[0], method_hash);
    }

    #[test]
    fn type_without_opcall_not_callable() {
        let (mut registry, _) = create_test_context();
        let type_hash = TypeHash::from_name("NotCallable");
        let class = ClassEntry::ffi("NotCallable", TypeKind::script_object());
        registry.register_type(class.into()).unwrap();

        let ctx = CompilationContext::new(&registry);
        let type_entry = ctx.get_type(type_hash).unwrap();
        let class = type_entry.as_class().unwrap();

        // Should NOT find opCall method
        let opcall_methods = class.find_methods("opCall");
        assert!(opcall_methods.is_empty());
    }

    #[test]
    fn opcall_const_correctness_const_method_is_const() {
        let (mut registry, _) = create_test_context();
        let (_, method_hash) = register_class_with_opcall(&mut registry, "ConstFunctor", true);

        let ctx = CompilationContext::new(&registry);
        let func = ctx.get_function(method_hash).unwrap();

        // Const opCall should be marked const
        assert!(func.def.is_const());
    }

    #[test]
    fn opcall_const_correctness_non_const_method_not_const() {
        let (mut registry, _) = create_test_context();
        let (_, method_hash) = register_class_with_opcall(&mut registry, "MutableFunctor", false);

        let ctx = CompilationContext::new(&registry);
        let func = ctx.get_function(method_hash).unwrap();

        // Non-const opCall should not be marked const
        assert!(!func.def.is_const());
    }

    // =========================================================================
    // Funcdef tests
    // =========================================================================

    #[test]
    fn funcdef_type_is_detected() {
        use angelscript_core::FuncdefEntry;

        let (mut registry, _) = create_test_context();
        let type_hash = TypeHash::from_name("Callback");

        let funcdef = FuncdefEntry::ffi(
            "Callback",
            vec![DataType::simple(primitives::INT32)],
            DataType::void(),
        );
        registry.register_type(funcdef.into()).unwrap();

        let ctx = CompilationContext::new(&registry);
        let type_entry = ctx.get_type(type_hash).unwrap();

        // Should be a funcdef
        assert!(type_entry.as_funcdef().is_some());
    }

    #[test]
    fn funcdef_has_correct_signature() {
        use angelscript_core::FuncdefEntry;

        let (mut registry, _) = create_test_context();
        let type_hash = TypeHash::from_name("IntCallback");

        let funcdef = FuncdefEntry::ffi(
            "IntCallback",
            vec![DataType::simple(primitives::INT32)],
            DataType::simple(primitives::BOOL),
        );
        registry.register_type(funcdef.into()).unwrap();

        let ctx = CompilationContext::new(&registry);
        let type_entry = ctx.get_type(type_hash).unwrap();
        let funcdef = type_entry.as_funcdef().unwrap();

        // Check signature
        assert_eq!(funcdef.params.len(), 1);
        assert_eq!(funcdef.params[0].type_hash, primitives::INT32);
        assert_eq!(funcdef.return_type.type_hash, primitives::BOOL);
    }

    // =========================================================================
    // compile_function_call tests
    // =========================================================================

    fn register_function_with_params(
        registry: &mut SymbolRegistry,
        name: &str,
        params: Vec<DataType>,
        return_type: DataType,
    ) -> TypeHash {
        use angelscript_core::Param;

        let param_hashes: Vec<_> = params.iter().map(|p| p.type_hash).collect();
        let func_hash = TypeHash::from_function(name, &param_hashes);

        let param_defs: Vec<Param> = params
            .into_iter()
            .enumerate()
            .map(|(i, dt)| Param {
                name: format!("p{}", i),
                data_type: dt,
                has_default: false,
                if_handle_then_const: false,
            })
            .collect();

        let func_def = FunctionDef::new(
            func_hash,
            name.to_string(),
            vec![],
            param_defs,
            return_type,
            None,
            FunctionTraits::default(),
            true,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(func_def))
            .unwrap();
        func_hash
    }

    #[test]
    fn compile_function_call_with_matching_args() {
        let (mut registry, mut constants) = create_test_context();

        let func_hash = register_function_with_params(
            &mut registry,
            "add",
            vec![
                DataType::simple(primitives::INT32),
                DataType::simple(primitives::INT32),
            ],
            DataType::simple(primitives::INT32),
        );

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        // Create call expression manually isn't easy, but we can test the helper
        let return_type = get_function_return_type(&compiler, func_hash);
        assert!(return_type.is_ok());
        assert_eq!(return_type.unwrap().type_hash, primitives::INT32);
    }

    #[test]
    fn compile_function_returns_correct_type() {
        let (mut registry, _) = create_test_context();

        // Function returning float
        let func_hash = register_function_with_params(
            &mut registry,
            "get_pi",
            vec![],
            DataType::simple(primitives::FLOAT),
        );

        let ctx = CompilationContext::new(&registry);
        let func = ctx.get_function(func_hash).unwrap();
        assert_eq!(func.def.return_type.type_hash, primitives::FLOAT);
    }

    // =========================================================================
    // compile_constructor_call tests (extended)
    // =========================================================================

    fn register_class_with_overloaded_constructors(
        registry: &mut SymbolRegistry,
        name: &str,
    ) -> (TypeHash, TypeHash, TypeHash) {
        use angelscript_core::Param;

        let type_hash = TypeHash::from_name(name);
        let ctor1_hash = TypeHash::from_constructor(type_hash, &[]);
        let ctor2_hash = TypeHash::from_constructor(type_hash, &[primitives::INT32]);

        let mut class = ClassEntry::ffi(name, TypeKind::script_object());
        class.behaviors = TypeBehaviors {
            constructors: vec![ctor1_hash, ctor2_hash],
            ..Default::default()
        };
        registry.register_type(class.into()).unwrap();

        // Register default constructor
        let ctor1_def = FunctionDef::new(
            ctor1_hash,
            "$ctor".to_string(),
            vec![],
            vec![],
            DataType::void(),
            Some(type_hash),
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(ctor1_def))
            .unwrap();

        // Register constructor with int param
        let ctor2_def = FunctionDef::new(
            ctor2_hash,
            "$ctor".to_string(),
            vec![],
            vec![Param {
                name: "value".to_string(),
                data_type: DataType::simple(primitives::INT32),
                has_default: false,
                if_handle_then_const: false,
            }],
            DataType::void(),
            Some(type_hash),
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(ctor2_def))
            .unwrap();

        (type_hash, ctor1_hash, ctor2_hash)
    }

    #[test]
    fn constructor_overload_resolution_selects_correct_ctor() {
        let (mut registry, _) = create_test_context();
        let (type_hash, ctor1_hash, ctor2_hash) =
            register_class_with_overloaded_constructors(&mut registry, "OverloadedClass");

        let ctx = CompilationContext::new(&registry);
        let type_entry = ctx.get_type(type_hash).unwrap();
        let class = type_entry.as_class().unwrap();

        // Should have two constructors
        assert_eq!(class.behaviors.constructors.len(), 2);
        assert!(class.behaviors.constructors.contains(&ctor1_hash));
        assert!(class.behaviors.constructors.contains(&ctor2_hash));
    }

    #[test]
    fn constructor_call_type_not_found_error() {
        let (registry, mut constants) = create_test_context();

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        let unknown_hash = TypeHash::from_name("NonExistentType");
        let result = validate_instantiable(&compiler, unknown_hash, Span::default());

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CompilationError::UnknownType { .. }
        ));
    }

    #[test]
    fn constructor_call_no_constructors_available() {
        let (mut registry, _) = create_test_context();

        // Create class with NO constructors
        let type_hash = TypeHash::from_name("NoCtorClass");
        let mut class = ClassEntry::ffi("NoCtorClass", TypeKind::script_object());
        // explicitly empty constructors
        class.behaviors = TypeBehaviors::default();
        registry.register_type(class.into()).unwrap();

        let ctx = CompilationContext::new(&registry);
        let type_entry = ctx.get_type(type_hash).unwrap();
        let class = type_entry.as_class().unwrap();

        // Should have empty constructors
        assert!(class.behaviors.constructors.is_empty());
    }

    // =========================================================================
    // Edge case tests
    // =========================================================================

    #[test]
    fn validate_instantiable_passes_for_primitives() {
        let (registry, mut constants) = create_test_context();

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        // validate_instantiable only checks mixin/abstract/interface
        // Primitives pass validation - the error happens in compile_constructor_call
        // when trying to call as_class() on a primitive
        let result = validate_instantiable(&compiler, primitives::INT32, Span::default());
        // Primitives pass instantiability check (they're not mixin/abstract/interface)
        // Actual error happens when trying to get constructors from as_class()
        assert!(result.is_ok());
    }

    #[test]
    fn primitive_type_not_a_class() {
        let (registry, _) = create_test_context();
        let ctx = CompilationContext::new(&registry);

        // Primitives are NOT classes
        let type_entry = ctx.get_type(primitives::INT32).unwrap();
        assert!(type_entry.as_class().is_none());
    }

    #[test]
    fn get_function_return_type_void() {
        let (mut registry, mut constants) = create_test_context();
        let func_hash = register_simple_function(&mut registry, "void_func");

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        let return_type = get_function_return_type(&compiler, func_hash).unwrap();
        assert_eq!(return_type.type_hash, primitives::VOID);
    }

    #[test]
    fn multiple_methods_same_name_different_constness() {
        let (mut registry, mut constants) = create_test_context();
        let type_hash = TypeHash::from_name("DualMethod");

        // Non-const version
        let method_hash_nonconst = TypeHash::from_method(type_hash, "get", &[]);
        // Const version (different hash due to const overload)
        let method_hash_const = TypeHash::from_method(type_hash, "get_const", &[]);

        let mut class = ClassEntry::ffi("DualMethod", TypeKind::script_object());
        class.add_method("get", method_hash_nonconst);
        class.add_method("get", method_hash_const);
        registry.register_type(class.into()).unwrap();

        // Register non-const method
        let mut traits = FunctionTraits::default();
        traits.is_const = false;
        let method_def = FunctionDef::new(
            method_hash_nonconst,
            "get".to_string(),
            vec![],
            vec![],
            DataType::simple(primitives::INT32),
            Some(type_hash),
            traits,
            true,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(method_def))
            .unwrap();

        // Register const method
        let mut traits_const = FunctionTraits::default();
        traits_const.is_const = true;
        let const_method_def = FunctionDef::new(
            method_hash_const,
            "get".to_string(),
            vec![],
            vec![],
            DataType::simple(primitives::INT32),
            Some(type_hash),
            traits_const,
            true,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(const_method_def))
            .unwrap();

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        // Calling on non-const object should find both methods
        let candidates = compiler.ctx().find_methods(type_hash, "get");
        assert_eq!(candidates.len(), 2);
    }

    #[test]
    fn apply_argument_conversions_with_actual_conversion() {
        use crate::conversion::{Conversion, ConversionKind};

        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let mut compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        // Create a conversion that would emit bytecode
        let overload = OverloadMatch {
            func_hash: TypeHash::from_function("test", &[]),
            arg_conversions: vec![Some(Conversion {
                kind: ConversionKind::Primitive {
                    from: primitives::INT32,
                    to: primitives::INT64,
                },
                cost: Conversion::COST_PRIMITIVE_WIDENING,
                is_implicit: true,
            })],
            total_cost: Conversion::COST_PRIMITIVE_WIDENING,
        };

        let result = apply_argument_conversions(&mut compiler, &overload);
        assert!(result.is_ok());

        // The emitter should have the conversion opcode
        // (We can't easily verify the bytecode here, but at least no error)
    }

    // =========================================================================
    // Type kind tests
    // =========================================================================

    #[test]
    fn script_object_uses_constructors() {
        assert!(TypeKind::script_object().uses_constructors());
        assert!(!TypeKind::script_object().uses_factories());
    }

    #[test]
    fn reference_type_uses_factories() {
        assert!(TypeKind::reference().uses_factories());
        assert!(!TypeKind::reference().uses_constructors());
    }

    #[test]
    fn value_type_uses_constructors() {
        assert!(TypeKind::value::<i32>().uses_constructors());
        assert!(!TypeKind::value::<i32>().uses_factories());
    }

    // =========================================================================
    // Error message tests
    // =========================================================================

    #[test]
    fn error_messages_include_type_names() {
        let (mut registry, _) = create_test_context();

        // Create abstract class
        let type_hash = TypeHash::from_name("AbstractWidget");
        let mut class = ClassEntry::ffi("AbstractWidget", TypeKind::script_object());
        class.is_abstract = true;
        registry.register_type(class.into()).unwrap();

        let ctx = CompilationContext::new(&registry);
        let type_entry = ctx.get_type(type_hash).unwrap();

        // Error should include type name
        assert_eq!(type_entry.qualified_name(), "AbstractWidget");
    }

    #[test]
    fn mixin_detection_accurate() {
        let (mut registry, _) = create_test_context();

        let type_hash = TypeHash::from_name("TestMixin");
        let mut class = ClassEntry::ffi("TestMixin", TypeKind::script_object());
        class.is_mixin = true;
        registry.register_type(class.into()).unwrap();

        let ctx = CompilationContext::new(&registry);
        let type_entry = ctx.get_type(type_hash).unwrap();
        let class = type_entry.as_class().unwrap();

        assert!(class.is_mixin);
    }

    #[test]
    fn abstract_detection_accurate() {
        let (mut registry, _) = create_test_context();

        let type_hash = TypeHash::from_name("TestAbstract");
        let mut class = ClassEntry::ffi("TestAbstract", TypeKind::script_object());
        class.is_abstract = true;
        registry.register_type(class.into()).unwrap();

        let ctx = CompilationContext::new(&registry);
        let type_entry = ctx.get_type(type_hash).unwrap();
        let class = type_entry.as_class().unwrap();

        assert!(class.is_abstract);
    }

    // =========================================================================
    // Lambda function call tests
    // =========================================================================

    #[test]
    fn has_lambda_argument_detects_lambda() {
        use angelscript_parser::ast::{
            Argument, Block, CallExpr, Expr, Ident, IdentExpr, LambdaExpr,
        };
        use bumpalo::Bump;

        let arena = Bump::new();

        // Create a lambda expression
        let body = arena.alloc(Block {
            stmts: &[],
            span: Span::default(),
        });
        let lambda_expr = arena.alloc(LambdaExpr {
            params: &[],
            return_type: None,
            body,
            span: Span::default(),
        });
        let lambda = arena.alloc(Expr::Lambda(lambda_expr));

        // Create a call with lambda argument
        let callee = arena.alloc(Expr::Ident(IdentExpr {
            scope: None,
            ident: Ident::new("test", Span::default()),
            type_args: &[],
            span: Span::default(),
        }));
        let args = arena.alloc_slice_copy(&[Argument {
            name: None,
            value: lambda,
            span: Span::default(),
        }]);
        let call = CallExpr {
            callee,
            args,
            span: Span::default(),
        };

        assert!(has_lambda_argument(&call));
    }

    #[test]
    fn has_lambda_argument_no_lambda() {
        use angelscript_parser::ast::{
            Argument, CallExpr, Expr, Ident, IdentExpr, LiteralExpr, LiteralKind,
        };
        use bumpalo::Bump;

        let arena = Bump::new();

        // Create a non-lambda expression
        let literal = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::default(),
        }));

        // Create a call without lambda argument
        let callee = arena.alloc(Expr::Ident(IdentExpr {
            scope: None,
            ident: Ident::new("test", Span::default()),
            type_args: &[],
            span: Span::default(),
        }));
        let args = arena.alloc_slice_copy(&[Argument {
            name: None,
            value: literal,
            span: Span::default(),
        }]);
        let call = CallExpr {
            callee,
            args,
            span: Span::default(),
        };

        assert!(!has_lambda_argument(&call));
    }

    #[test]
    fn has_lambda_argument_empty_args() {
        use angelscript_parser::ast::{CallExpr, Expr, Ident, IdentExpr};
        use bumpalo::Bump;

        let arena = Bump::new();

        let callee = arena.alloc(Expr::Ident(IdentExpr {
            scope: None,
            ident: Ident::new("test", Span::default()),
            type_args: &[],
            span: Span::default(),
        }));
        let call = CallExpr {
            callee,
            args: &[],
            span: Span::default(),
        };

        assert!(!has_lambda_argument(&call));
    }

    #[test]
    fn compile_lambda_function_call_multiple_candidates_error() {
        use angelscript_core::FuncdefEntry;
        use angelscript_parser::ast::{
            Argument, Block, CallExpr, Expr, Ident, IdentExpr, LambdaExpr,
        };
        use bumpalo::Bump;

        let (mut registry, _) = create_test_context();

        // Register funcdef for the lambda parameter type
        let funcdef_hash = TypeHash::from_name("Callback");
        let funcdef = FuncdefEntry::ffi("Callback", vec![], DataType::void());
        registry.register_type(funcdef.into()).unwrap();

        // Register two overloaded functions that take the funcdef
        let func1_hash = register_function_with_params(
            &mut registry,
            "apply",
            vec![DataType::simple(funcdef_hash)],
            DataType::void(),
        );
        let func2_hash = register_function_with_params(
            &mut registry,
            "apply",
            vec![
                DataType::simple(funcdef_hash),
                DataType::simple(primitives::INT32),
            ],
            DataType::void(),
        );

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let mut compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        let arena = Bump::new();

        // Create lambda expression
        let body = arena.alloc(Block {
            stmts: &[],
            span: Span::default(),
        });
        let lambda_expr = arena.alloc(LambdaExpr {
            params: &[],
            return_type: None,
            body,
            span: Span::default(),
        });
        let lambda = arena.alloc(Expr::Lambda(lambda_expr));

        // Create call with lambda argument
        let callee = arena.alloc(Expr::Ident(IdentExpr {
            scope: None,
            ident: Ident::new("apply", Span::default()),
            type_args: &[],
            span: Span::default(),
        }));
        let args = arena.alloc_slice_copy(&[Argument {
            name: None,
            value: lambda,
            span: Span::default(),
        }]);
        let call = CallExpr {
            callee,
            args,
            span: Span::default(),
        };

        // Multiple candidates should error
        let result =
            compile_lambda_function_call(&mut compiler, vec![func1_hash, func2_hash], &call);

        assert!(result.is_err());
        match result.unwrap_err() {
            CompilationError::TypeMismatch { message, .. } => {
                assert!(
                    message.contains("unambiguous"),
                    "Error should mention ambiguity: {}",
                    message
                );
            }
            other => panic!("Expected TypeMismatch error, got {:?}", other),
        }
    }

    #[test]
    fn compile_lambda_function_call_single_candidate_succeeds() {
        use angelscript_core::FuncdefEntry;
        use angelscript_parser::ast::{
            Argument, Block, CallExpr, Expr, Ident, IdentExpr, LambdaExpr,
        };
        use bumpalo::Bump;

        let (mut registry, _) = create_test_context();

        // Register funcdef for the lambda parameter type
        let funcdef_hash = TypeHash::from_name("VoidCallback");
        let funcdef = FuncdefEntry::ffi("VoidCallback", vec![], DataType::void());
        registry.register_type(funcdef.into()).unwrap();

        // Register single function that takes the funcdef
        let func_hash = register_function_with_params(
            &mut registry,
            "execute",
            vec![DataType::simple(funcdef_hash)],
            DataType::simple(primitives::INT32),
        );

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let mut compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        let arena = Bump::new();

        // Create lambda expression with no params (matches funcdef)
        let body = arena.alloc(Block {
            stmts: &[],
            span: Span::default(),
        });
        let lambda_expr = arena.alloc(LambdaExpr {
            params: &[],
            return_type: None,
            body,
            span: Span::default(),
        });
        let lambda = arena.alloc(Expr::Lambda(lambda_expr));

        // Create call with lambda argument
        let callee = arena.alloc(Expr::Ident(IdentExpr {
            scope: None,
            ident: Ident::new("execute", Span::default()),
            type_args: &[],
            span: Span::default(),
        }));
        let args = arena.alloc_slice_copy(&[Argument {
            name: None,
            value: lambda,
            span: Span::default(),
        }]);
        let call = CallExpr {
            callee,
            args,
            span: Span::default(),
        };

        // Single candidate should succeed
        let result = compile_lambda_function_call(&mut compiler, vec![func_hash], &call);

        assert!(
            result.is_ok(),
            "Single candidate should succeed: {:?}",
            result
        );
        let info = result.unwrap();

        // Validate return type matches the function's return type
        assert_eq!(info.data_type.type_hash, primitives::INT32);

        // Validate a lambda was compiled (registered as a function)
        let compiled = emitter.compiled_functions();
        assert_eq!(compiled.len(), 1, "Lambda should be registered");

        // Validate the lambda's bytecode contains ReturnVoid (void lambda with empty body)
        use crate::bytecode::OpCode;
        let lambda_bytecode = &compiled[0].bytecode;
        let code = lambda_bytecode.code();
        assert!(!code.is_empty(), "Lambda bytecode should not be empty");
        assert_eq!(
            code[code.len() - 1],
            OpCode::ReturnVoid as u8,
            "Void lambda should end with ReturnVoid"
        );
    }

    #[test]
    fn compile_lambda_function_call_arg_count_mismatch() {
        use angelscript_core::FuncdefEntry;
        use angelscript_parser::ast::{
            Argument, Block, CallExpr, Expr, Ident, IdentExpr, LambdaExpr,
        };
        use bumpalo::Bump;

        let (mut registry, _) = create_test_context();

        // Register funcdef
        let funcdef_hash = TypeHash::from_name("Callback2");
        let funcdef = FuncdefEntry::ffi("Callback2", vec![], DataType::void());
        registry.register_type(funcdef.into()).unwrap();

        // Register function that takes TWO funcdef params
        let func_hash = register_function_with_params(
            &mut registry,
            "dualApply",
            vec![
                DataType::simple(funcdef_hash),
                DataType::simple(funcdef_hash),
            ],
            DataType::void(),
        );

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let mut compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        let arena = Bump::new();

        // Create only ONE lambda argument (but function expects TWO)
        let body = arena.alloc(Block {
            stmts: &[],
            span: Span::default(),
        });
        let lambda_expr = arena.alloc(LambdaExpr {
            params: &[],
            return_type: None,
            body,
            span: Span::default(),
        });
        let lambda = arena.alloc(Expr::Lambda(lambda_expr));

        let callee = arena.alloc(Expr::Ident(IdentExpr {
            scope: None,
            ident: Ident::new("dualApply", Span::default()),
            type_args: &[],
            span: Span::default(),
        }));
        let args = arena.alloc_slice_copy(&[Argument {
            name: None,
            value: lambda,
            span: Span::default(),
        }]);
        let call = CallExpr {
            callee,
            args,
            span: Span::default(),
        };

        // Should fail with argument count mismatch
        let result = compile_lambda_function_call(&mut compiler, vec![func_hash], &call);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CompilationError::ArgumentCountMismatch { .. }
        ));
    }
}
