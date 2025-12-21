//! Assignment expression compilation.
//!
//! Compiles assignment expressions:
//! - Simple assignment: `a = b`
//! - Compound assignment: `a += b`, `a -= b`, etc.
//! - Member assignment: `obj.field = value`
//! - Index assignment: `arr[i] = value`

use angelscript_core::{CompilationError, DataType, OperatorBehavior, Span, TypeHash};
use angelscript_parser::ast::{AssignExpr, AssignOp, BinaryOp, Expr, MemberAccess, UnaryOp};

use super::{ExprCompiler, Result};
use crate::bytecode::OpCode;
use crate::expr_info::ExprInfo;
use crate::operators::{OperatorResolution, resolve_binary};

/// Compile an assignment expression.
pub fn compile_assign<'ast>(
    compiler: &mut ExprCompiler<'_, '_>,
    assign: &AssignExpr<'ast>,
) -> Result<ExprInfo> {
    let span = assign.span;

    match assign.op {
        AssignOp::Assign => compile_simple_assign(compiler, assign, span),
        _ => compile_compound_assign(compiler, assign, span),
    }
}

/// Compile a simple assignment (`a = b`).
fn compile_simple_assign<'ast>(
    compiler: &mut ExprCompiler<'_, '_>,
    assign: &AssignExpr<'ast>,
    span: Span,
) -> Result<ExprInfo> {
    // For index expressions, we need to compile the value first to know its type
    // for overload resolution of set_opIndex(index, value).
    if let Expr::Index(index) = assign.target {
        return compile_index_simple_assign(compiler, index, assign.value, span);
    }

    // Analyze target to get its type and storage kind
    let target = analyze_assign_target(compiler, assign.target, span)?;

    // Check that target is assignable
    if !target.is_mutable {
        return Err(CompilationError::CannotModifyConst {
            message: "cannot assign to const variable".to_string(),
            span,
        });
    }

    // For IndexRef, we need to call opIndex to get the reference before compiling value
    if let AssignTargetKind::IndexRef {
        method_hash,
        index_count,
    } = &target.kind
    {
        // Stack: [obj, idx...]
        // Call opIndex to get reference
        compiler
            .emitter()
            .emit_call_method(*method_hash, *index_count);
        // Stack: [ref]
    }

    // Compile the value expression with type checking against target type
    let _value_info = compiler.check(assign.value, &target.data_type)?;

    // Emit store instruction based on target kind
    emit_store(compiler, &target)?;

    // Assignment expression returns the assigned value as rvalue
    Ok(ExprInfo::rvalue(target.data_type))
}

/// Compile simple assignment to an index expression.
///
/// For set_opIndex style, we need the value type for overload resolution.
/// The order is:
/// 1. Compile object (onto stack)
/// 2. Compile indices (onto stack, and collect types)
/// 3. Compile value (just infer type, don't emit yet)
/// 4. Resolve set_opIndex with all types [indices..., value]
/// 5. Emit value with type checking
/// 6. Call set_opIndex
fn compile_index_simple_assign<'ast>(
    compiler: &mut ExprCompiler<'_, '_>,
    index: &angelscript_parser::ast::IndexExpr<'ast>,
    value: &Expr<'ast>,
    span: Span,
) -> Result<ExprInfo> {
    // Compile the object expression (emits code to load the object)
    let obj_info = compiler.infer(index.object)?;

    // Compile all index expressions and collect their types
    let mut index_types = Vec::with_capacity(index.indices.len());
    for idx_item in index.indices {
        let info = compiler.infer(idx_item.index)?;
        index_types.push(info.data_type);
    }

    // Stack is now: [obj, idx0, idx1, ...]

    // Get class info
    let class = compiler
        .ctx()
        .get_type(obj_info.data_type.type_hash)
        .and_then(|e| e.as_class())
        .ok_or_else(|| CompilationError::Other {
            message: "cannot index non-class type".to_string(),
            span,
        })?;

    let class_name = class.qualified_name.clone();
    // Look up set_opIndex from behaviors, falling back to methods for backwards compatibility
    let set_opindex_methods = class
        .behaviors
        .get_operator(OperatorBehavior::OpIndexSet)
        .map(|v| v.to_vec())
        .or_else(|| {
            let methods = class.find_methods("set_opIndex");
            if methods.is_empty() {
                None
            } else {
                Some(methods.to_vec())
            }
        })
        .unwrap_or_default();

    if !set_opindex_methods.is_empty() {
        // IndexSetter style: set_opIndex(indices..., value)
        // Compile value now to get its type and emit its code
        // Stack will be: [obj, idx..., value]
        let value_info = compiler.infer(value)?;

        // Build full argument types: [indices..., value]
        let mut all_arg_types = index_types.clone();
        all_arg_types.push(value_info.data_type);

        use crate::overload::resolve_overload;
        let overload =
            resolve_overload(&set_opindex_methods, &all_arg_types, compiler.ctx(), span)?;

        // Check mutability
        let is_const = obj_info.data_type.is_effectively_const();
        if is_const {
            return Err(CompilationError::CannotModifyConst {
                message: "cannot assign to index on const object".to_string(),
                span,
            });
        }

        // Get the value type from the resolved overload for return
        let value_type = {
            let func = compiler
                .ctx()
                .get_function(overload.func_hash)
                .ok_or_else(|| CompilationError::Internal {
                    message: format!("set_opIndex method not found: {:?}", overload.func_hash),
                })?;

            func.def.params.last().map(|p| p.data_type).ok_or_else(|| {
                CompilationError::Internal {
                    message: "set_opIndex must have at least one parameter".to_string(),
                }
            })?
        };

        // Stack is now: [obj, idx0, idx1, ..., value]
        // Call set_opIndex
        let arg_count = index_types.len() as u8 + 1; // indices + value
        compiler
            .emitter()
            .emit_call_method(overload.func_hash, arg_count);

        Ok(ExprInfo::rvalue(value_type))
    } else {
        // Check for opIndex that returns a reference
        let op_index_methods = {
            let class = compiler
                .ctx()
                .get_type(obj_info.data_type.type_hash)
                .and_then(|e| e.as_class())
                .ok_or_else(|| CompilationError::Internal {
                    message: "type not found".to_string(),
                })?;
            // Look up opIndex from behaviors, falling back to methods
            class
                .behaviors
                .get_operator(OperatorBehavior::OpIndex)
                .map(|v| v.to_vec())
                .or_else(|| {
                    let methods = class.find_methods("opIndex");
                    if methods.is_empty() {
                        None
                    } else {
                        Some(methods.to_vec())
                    }
                })
                .unwrap_or_default()
        };

        if !op_index_methods.is_empty() {
            use crate::overload::resolve_overload;
            // Filter to non-const methods for assignment (mutable opIndex)
            let is_const_obj = obj_info.data_type.is_effectively_const();
            let mutable_candidates: Vec<TypeHash> = op_index_methods
                .iter()
                .filter(|&&hash| {
                    compiler
                        .ctx()
                        .get_function(hash)
                        .is_some_and(|f| !f.def.is_const())
                })
                .copied()
                .collect();

            if mutable_candidates.is_empty() {
                return Err(CompilationError::CannotModifyConst {
                    message: format!("cannot assign via const opIndex on type '{}'", class_name),
                    span,
                });
            }

            if is_const_obj {
                return Err(CompilationError::CannotModifyConst {
                    message: "cannot assign to index on const object".to_string(),
                    span,
                });
            }

            let overload =
                resolve_overload(&mutable_candidates, &index_types, compiler.ctx(), span)?;

            let return_type = compiler
                .ctx()
                .get_function(overload.func_hash)
                .ok_or_else(|| CompilationError::Internal {
                    message: format!("opIndex method not found: {:?}", overload.func_hash),
                })?
                .def
                .return_type;

            // Call opIndex to get reference
            // Stack: [obj, idx...] -> [ref]
            compiler
                .emitter()
                .emit_call_method(overload.func_hash, index_types.len() as u8);

            // Compile value with type checking
            compiler.check(value, &return_type)?;

            // Stack: [ref, value]
            // Store through reference
            compiler.emitter().emit(OpCode::Swap);
            compiler.emitter().emit_set_field(0);

            Ok(ExprInfo::rvalue(return_type))
        } else {
            Err(CompilationError::Other {
                message: format!(
                    "type '{}' does not support index assignment (no set_opIndex or opIndex)",
                    class_name
                ),
                span,
            })
        }
    }
}

/// Compile a compound assignment (`a += b`, etc.).
fn compile_compound_assign<'ast>(
    compiler: &mut ExprCompiler<'_, '_>,
    assign: &AssignExpr<'ast>,
    span: Span,
) -> Result<ExprInfo> {
    // For index expressions, we need special handling for set_opIndex overload resolution
    if let Expr::Index(index) = assign.target {
        return compile_index_compound_assign(compiler, index, assign.op, assign.value, span);
    }

    // For compound assignment (a += b):
    // 1. Analyze target
    // 2. Load current value of target
    // 3. Compile value expression
    // 4. Apply binary operation
    // 5. Store result back

    let target = analyze_assign_target(compiler, assign.target, span)?;

    // Check that target is assignable
    if !target.is_mutable {
        return Err(CompilationError::CannotModifyConst {
            message: format!("cannot use '{}' on const variable", assign.op),
            span,
        });
    }

    // Load current value of target
    emit_load(compiler, &target, span)?;

    // Compile value expression (infer its type first)
    let value_info = compiler.infer(assign.value)?;

    // Get the binary operator for this compound assignment
    let binary_op = compound_to_binary_op(assign.op, span)?;

    // Resolve the binary operation
    let resolution = resolve_binary(
        &target.data_type,
        &value_info.data_type,
        binary_op,
        compiler.ctx(),
        span,
    )?;

    // Emit the binary operation
    emit_binary_op(compiler, &resolution, span)?;

    // Store result back to target
    emit_store(compiler, &target)?;

    // Compound assignment returns the result as rvalue
    Ok(ExprInfo::rvalue(target.data_type))
}

/// Compile compound assignment to an index expression.
///
/// For set_opIndex style (container[i] += x):
/// 1. Compile object and indices
/// 2. Duplicate obj+indices for getter call
/// 3. Call getter to get current value
/// 4. Compile RHS value
/// 5. Apply binary operation
/// 6. Resolve set_opIndex with result type
/// 7. Call set_opIndex
fn compile_index_compound_assign<'ast>(
    compiler: &mut ExprCompiler<'_, '_>,
    index: &angelscript_parser::ast::IndexExpr<'ast>,
    op: AssignOp,
    value: &Expr<'ast>,
    span: Span,
) -> Result<ExprInfo> {
    // Compile the object expression (emits code to load the object)
    let obj_info = compiler.infer(index.object)?;

    // Compile all index expressions and collect their types
    let mut index_types = Vec::with_capacity(index.indices.len());
    for idx_item in index.indices {
        let info = compiler.infer(idx_item.index)?;
        index_types.push(info.data_type);
    }

    // Stack is now: [obj, idx0, idx1, ...]

    // Get class info
    let class = compiler
        .ctx()
        .get_type(obj_info.data_type.type_hash)
        .and_then(|e| e.as_class())
        .ok_or_else(|| CompilationError::Other {
            message: "cannot index non-class type".to_string(),
            span,
        })?;

    let class_name = class.qualified_name.clone();
    // Look up set_opIndex from behaviors, falling back to methods for backwards compatibility
    let set_opindex_methods = class
        .behaviors
        .get_operator(OperatorBehavior::OpIndexSet)
        .map(|v| v.to_vec())
        .or_else(|| {
            let methods = class.find_methods("set_opIndex");
            if methods.is_empty() {
                None
            } else {
                Some(methods.to_vec())
            }
        })
        .unwrap_or_default();

    if !set_opindex_methods.is_empty() {
        // IndexSetter style: need getter first, then setter
        // Find getter (get_opIndex or opIndex)
        let (getter_methods, getter_name) = {
            let class = compiler
                .ctx()
                .get_type(obj_info.data_type.type_hash)
                .and_then(|e| e.as_class())
                .ok_or_else(|| CompilationError::Internal {
                    message: "type not found".to_string(),
                })?;

            let get_methods = class.find_methods("get_opIndex").to_vec();
            if !get_methods.is_empty() {
                (get_methods, "get_opIndex")
            } else {
                (class.find_methods("opIndex").to_vec(), "opIndex")
            }
        };

        if getter_methods.is_empty() {
            return Err(CompilationError::Other {
                message: format!(
                    "compound assignment on '{}' requires get_opIndex or opIndex",
                    class_name
                ),
                span,
            });
        }

        use crate::overload::resolve_overload;

        // Resolve getter
        let getter_overload = resolve_overload(&getter_methods, &index_types, compiler.ctx(), span)
            .map_err(|_| CompilationError::Other {
                message: format!("no matching {} for compound assignment", getter_name),
                span,
            })?;

        let current_value_type = {
            let func = compiler
                .ctx()
                .get_function(getter_overload.func_hash)
                .ok_or_else(|| CompilationError::Internal {
                    message: format!("{} method not found", getter_name),
                })?;
            func.def.return_type
        };

        // Duplicate obj+indices for getter call
        // Stack: [obj, idx0, idx1, ...]
        // After dup: [obj, idx..., obj, idx...]
        let total = index_types.len() as u8 + 1; // object + indices
        for i in (0..total).rev() {
            compiler.emitter().emit_pick(total - 1 + (total - 1 - i));
        }

        // Call getter
        // Stack: [obj, idx..., obj, idx...] -> [obj, idx..., current_value]
        compiler
            .emitter()
            .emit_call_method(getter_overload.func_hash, index_types.len() as u8);

        // Compile RHS value
        let rhs_info = compiler.infer(value)?;

        // Get binary operator
        let binary_op = compound_to_binary_op(op, span)?;

        // Resolve binary operation
        let resolution = resolve_binary(
            &current_value_type,
            &rhs_info.data_type,
            binary_op,
            compiler.ctx(),
            span,
        )?;

        // Emit binary operation
        // Stack: [obj, idx..., current_value, rhs] -> [obj, idx..., new_value]
        emit_binary_op(compiler, &resolution, span)?;

        // Get the result type after binary operation
        let result_type = resolution.result_type();

        // Resolve set_opIndex with actual result type
        let mut all_arg_types = index_types.clone();
        all_arg_types.push(result_type);

        let setter_overload =
            resolve_overload(&set_opindex_methods, &all_arg_types, compiler.ctx(), span)?;

        // Check mutability
        let is_const = obj_info.data_type.is_effectively_const();
        if is_const {
            return Err(CompilationError::CannotModifyConst {
                message: "cannot assign to index on const object".to_string(),
                span,
            });
        }

        // Call set_opIndex
        // Stack: [obj, idx..., new_value]
        let arg_count = index_types.len() as u8 + 1;
        compiler
            .emitter()
            .emit_call_method(setter_overload.func_hash, arg_count);

        Ok(ExprInfo::rvalue(result_type))
    } else {
        // Check for opIndex that returns a reference
        let op_index_methods = {
            let class = compiler
                .ctx()
                .get_type(obj_info.data_type.type_hash)
                .and_then(|e| e.as_class())
                .ok_or_else(|| CompilationError::Internal {
                    message: "type not found".to_string(),
                })?;
            class.find_methods("opIndex").to_vec()
        };

        if !op_index_methods.is_empty() {
            use crate::overload::resolve_overload;
            let overload = resolve_overload(&op_index_methods, &index_types, compiler.ctx(), span)?;

            let func = compiler
                .ctx()
                .get_function(overload.func_hash)
                .ok_or_else(|| CompilationError::Internal {
                    message: format!("opIndex method not found: {:?}", overload.func_hash),
                })?;

            let return_type = func.def.return_type;
            let is_const_method = func.def.traits.is_const;

            if is_const_method {
                return Err(CompilationError::CannotModifyConst {
                    message: format!("cannot assign via const opIndex on type '{}'", class_name),
                    span,
                });
            }

            let is_const_obj = obj_info.data_type.is_effectively_const();
            if is_const_obj {
                return Err(CompilationError::CannotModifyConst {
                    message: "cannot assign to index on const object".to_string(),
                    span,
                });
            }

            // Call opIndex to get reference
            // Stack: [obj, idx...] -> [ref]
            compiler
                .emitter()
                .emit_call_method(overload.func_hash, index_types.len() as u8);

            // Dup reference (need it for store later)
            compiler.emitter().emit_dup();

            // Load current value through reference
            // Stack: [ref, ref] -> [ref, current_value]
            compiler.emitter().emit_get_field(0);

            // Compile RHS value
            let rhs_info = compiler.infer(value)?;

            // Get binary operator
            let binary_op = compound_to_binary_op(op, span)?;

            // Resolve and emit binary operation
            let resolution = resolve_binary(
                &return_type,
                &rhs_info.data_type,
                binary_op,
                compiler.ctx(),
                span,
            )?;

            emit_binary_op(compiler, &resolution, span)?;

            // Stack: [ref, new_value]
            // Store through reference
            compiler.emitter().emit(OpCode::Swap);
            compiler.emitter().emit_set_field(0);

            Ok(ExprInfo::rvalue(return_type))
        } else {
            Err(CompilationError::Other {
                message: format!(
                    "type '{}' does not support index assignment (no set_opIndex or opIndex)",
                    class_name
                ),
                span,
            })
        }
    }
}

/// Information about an assignment target.
#[derive(Debug)]
struct AssignTarget {
    /// The type of the target.
    data_type: DataType,
    /// Whether the target is mutable.
    is_mutable: bool,
    /// The kind of target (determines store instruction).
    kind: AssignTargetKind,
}

/// Kind of assignment target.
#[derive(Debug)]
enum AssignTargetKind {
    /// Local variable.
    Local { slot: u32 },
    /// Global variable.
    Global { hash: TypeHash },
    /// Object field.
    Field { field_index: u16 },
    /// Virtual property with getter/setter methods.
    VirtualProperty {
        /// Setter method hash.
        setter_hash: TypeHash,
        /// Getter method hash (for compound assignment).
        getter_hash: Option<TypeHash>,
    },
    /// Index access via set_opIndex (property accessor style).
    IndexSetter {
        /// set_opIndex method hash for storing.
        setter_hash: TypeHash,
        /// get_opIndex method hash for loading (compound assignment).
        getter_hash: Option<TypeHash>,
        /// Number of index arguments (not including value).
        index_count: u8,
    },
    /// Index access via opIndex returning reference (direct style).
    /// The opIndex method returns a reference that can be assigned to.
    IndexRef {
        /// opIndex method hash (returns reference).
        method_hash: TypeHash,
        /// Number of index arguments.
        index_count: u8,
    },
}

/// Analyze the target of an assignment to determine its type and storage location.
///
/// This function does NOT emit code for loading the target - it only analyzes it.
fn analyze_assign_target<'ast>(
    compiler: &mut ExprCompiler<'_, '_>,
    target: &Expr<'ast>,
    span: Span,
) -> Result<AssignTarget> {
    match target {
        Expr::Ident(ident) => analyze_ident_target(compiler, ident, span),
        Expr::Member(member) => analyze_member_target(compiler, member, span),
        Expr::Index(index) => analyze_index_target(compiler, index, span),
        Expr::Unary(unary) if unary.op == UnaryOp::HandleOf => {
            analyze_handle_assign_target(compiler, unary.operand, span)
        }
        _ => Err(CompilationError::Other {
            message: "invalid assignment target".to_string(),
            span,
        }),
    }
}

/// Analyze a handle assignment target (`@var = value`).
///
/// When the `@` operator is used on the left side of an assignment,
/// it means we're assigning to the handle itself, not dereferencing it.
/// The operand must be a valid assignment target that holds a handle type.
fn analyze_handle_assign_target<'ast>(
    compiler: &mut ExprCompiler<'_, '_>,
    operand: &Expr<'ast>,
    span: Span,
) -> Result<AssignTarget> {
    // Recursively analyze the operand (must be ident, member, or index)
    let target = analyze_assign_target(compiler, operand, span)?;

    // The target must be a handle type for @target = value to make sense
    if !target.data_type.is_handle {
        return Err(CompilationError::Other {
            message: "handle assignment target must be a handle type".to_string(),
            span,
        });
    }

    // Return the same target - handle assignment works the same as regular assignment
    // The @ on the left side just confirms we're assigning the handle, not dereferencing
    Ok(target)
}

/// Analyze an identifier as an assignment target.
fn analyze_ident_target(
    compiler: &mut ExprCompiler<'_, '_>,
    ident: &angelscript_parser::ast::IdentExpr<'_>,
    span: Span,
) -> Result<AssignTarget> {
    let name = ident.ident.name;

    // Check for 'this' keyword - cannot assign to 'this'
    if name == "this" {
        return Err(CompilationError::Other {
            message: "cannot assign to 'this'".to_string(),
            span,
        });
    }

    // Build qualified name if scope is present
    let qualified_name = super::identifiers::build_qualified_name(ident);

    // Check local scope first (only for unqualified names)
    if ident.scope.is_none()
        && let Some(lookup) = compiler.ctx_mut().get_local_or_capture(name)
    {
        match lookup {
            crate::scope::VarLookup::Local(var) => {
                return Ok(AssignTarget {
                    data_type: var.data_type,
                    is_mutable: !var.is_const,
                    kind: AssignTargetKind::Local { slot: var.slot },
                });
            }
            crate::scope::VarLookup::Captured(_captured) => {
                // Captured variables cannot be assigned in closures (for now)
                return Err(CompilationError::Other {
                    message: format!("cannot assign to captured variable '{}'", name),
                    span,
                });
            }
        }
    }

    // Check for globals
    if let Some(global_hash) = compiler.ctx().resolve_global(&qualified_name)
        && let Some(global_entry) = compiler.ctx().get_global_entry(global_hash)
    {
        return Ok(AssignTarget {
            data_type: global_entry.data_type,
            is_mutable: !global_entry.is_const,
            kind: AssignTargetKind::Global { hash: global_hash },
        });
    }

    // Check if we're inside a class and the identifier is a member field (implicit this.field)
    if ident.scope.is_none()
        && let Some(class_hash) = compiler.current_class()
    {
        // Extract field info before mutably borrowing compiler
        let field_info = compiler
            .ctx()
            .get_type(class_hash)
            .and_then(|e| e.as_class())
            .and_then(|class| {
                class
                    .properties
                    .iter()
                    .enumerate()
                    .find(|(_, p)| p.name == name)
                    .map(|(idx, p)| (idx, p.clone()))
            });

        if let Some((field_idx, property)) = field_info {
            // Get 'this' const status from the declared parameter
            let this_is_const = compiler
                .ctx()
                .get_local("this")
                .map(|v| v.is_const)
                .unwrap_or(false);

            // Emit GetThis now - puts object on stack for Field/VirtualProperty handling
            compiler.emitter().emit_get_this();

            if property.is_direct_field() {
                return Ok(AssignTarget {
                    data_type: property.data_type,
                    is_mutable: !this_is_const,
                    kind: AssignTargetKind::Field {
                        field_index: field_idx as u16,
                    },
                });
            } else if let Some(setter_hash) = property.setter {
                return Ok(AssignTarget {
                    data_type: property.data_type,
                    is_mutable: !this_is_const,
                    kind: AssignTargetKind::VirtualProperty {
                        setter_hash,
                        getter_hash: property.getter,
                    },
                });
            } else {
                return Err(CompilationError::CannotModifyConst {
                    message: format!("property '{}' is read-only", name),
                    span,
                });
            }
        }
    }

    Err(CompilationError::UndefinedVariable {
        name: qualified_name,
        span,
    })
}

/// Analyze a member access as an assignment target.
fn analyze_member_target(
    compiler: &mut ExprCompiler<'_, '_>,
    member: &angelscript_parser::ast::MemberExpr<'_>,
    span: Span,
) -> Result<AssignTarget> {
    // Compile the object expression first (this will emit code to load the object)
    let obj_info = compiler.infer(member.object)?;

    match &member.member {
        MemberAccess::Field(field_ident) => {
            let field_name = field_ident.name;

            // Get the class entry
            let class = compiler
                .ctx()
                .get_type(obj_info.data_type.type_hash)
                .and_then(|e| e.as_class())
                .ok_or_else(|| CompilationError::Other {
                    message: format!("cannot access field '{}' on non-class type", field_name),
                    span,
                })?;

            // Find the property
            let (field_index, property) = class
                .properties
                .iter()
                .enumerate()
                .find(|(_, p)| p.name == field_name)
                .map(|(i, p)| (i, p.clone()))
                .ok_or_else(|| CompilationError::UnknownField {
                    field: field_name.to_string(),
                    type_name: class.qualified_name.clone(),
                    span,
                })?;

            // Check if it's a direct field (can be assigned to)
            if property.is_direct_field() {
                let is_const = obj_info.data_type.is_effectively_const();
                Ok(AssignTarget {
                    data_type: property.data_type,
                    is_mutable: !is_const,
                    kind: AssignTargetKind::Field {
                        field_index: field_index as u16,
                    },
                })
            } else if let Some(setter_hash) = property.setter {
                // Virtual property with setter
                let is_const = obj_info.data_type.is_effectively_const();
                Ok(AssignTarget {
                    data_type: property.data_type,
                    is_mutable: !is_const,
                    kind: AssignTargetKind::VirtualProperty {
                        setter_hash,
                        getter_hash: property.getter,
                    },
                })
            } else {
                // Read-only property (has getter but no setter)
                Err(CompilationError::CannotModifyConst {
                    message: format!("property '{}' is read-only", field_name),
                    span,
                })
            }
        }
        MemberAccess::Method { name, .. } => Err(CompilationError::Other {
            message: format!("cannot assign to method call '{}'", name.name),
            span,
        }),
    }
}

/// Analyze an index expression as an assignment target.
fn analyze_index_target(
    compiler: &mut ExprCompiler<'_, '_>,
    index: &angelscript_parser::ast::IndexExpr<'_>,
    span: Span,
) -> Result<AssignTarget> {
    // Compile the object expression (emits code to load the object)
    let obj_info = compiler.infer(index.object)?;

    // Compile all index expressions
    let mut index_types = Vec::with_capacity(index.indices.len());
    for idx_item in index.indices {
        let info = compiler.infer(idx_item.index)?;
        index_types.push(info.data_type);
    }

    // Look for set_opIndex method
    let (set_opindex_methods, class_name) = {
        let class = compiler
            .ctx()
            .get_type(obj_info.data_type.type_hash)
            .and_then(|e| e.as_class())
            .ok_or_else(|| CompilationError::Other {
                message: "cannot index non-class type".to_string(),
                span,
            })?;

        // Look up set_opIndex from behaviors, falling back to methods for backwards compatibility
        let set_ops = class
            .behaviors
            .get_operator(OperatorBehavior::OpIndexSet)
            .map(|v| v.to_vec())
            .or_else(|| {
                let methods = class.find_methods("set_opIndex");
                if methods.is_empty() {
                    None
                } else {
                    Some(methods.to_vec())
                }
            })
            .unwrap_or_default();

        (set_ops, class.qualified_name.clone())
    };

    if !set_opindex_methods.is_empty() {
        // Resolve overload for set_opIndex
        // The value being assigned will be the last argument
        use crate::overload::resolve_overload;

        // Try to find a matching set_opIndex
        // Note: set_opIndex takes (index..., value) so we need to check if there's a match
        let overload = resolve_overload(&set_opindex_methods, &index_types, compiler.ctx(), span)?;

        // Get the value type (last parameter of set_opIndex)
        let value_type = {
            let func = compiler
                .ctx()
                .get_function(overload.func_hash)
                .ok_or_else(|| CompilationError::Internal {
                    message: format!("set_opIndex method not found: {:?}", overload.func_hash),
                })?;

            // Last parameter is the value to set
            func.def.params.last().map(|p| p.data_type).ok_or_else(|| {
                CompilationError::Internal {
                    message: "set_opIndex must have at least one parameter".to_string(),
                }
            })?
        };

        // Look for corresponding getter (get_opIndex or opIndex) for compound assignment
        let getter_hash = {
            let class = compiler
                .ctx()
                .get_type(obj_info.data_type.type_hash)
                .and_then(|e| e.as_class())
                .ok_or_else(|| CompilationError::Internal {
                    message: "type not found".to_string(),
                })?;

            // Try get_opIndex first, then opIndex
            let get_opindex_methods = class.find_methods("get_opIndex");
            let opindex_methods = class.find_methods("opIndex");

            if !get_opindex_methods.is_empty() {
                resolve_overload(get_opindex_methods, &index_types, compiler.ctx(), span)
                    .ok()
                    .map(|o| o.func_hash)
            } else if !opindex_methods.is_empty() {
                resolve_overload(opindex_methods, &index_types, compiler.ctx(), span)
                    .ok()
                    .map(|o| o.func_hash)
            } else {
                None
            }
        };

        let is_const = obj_info.data_type.is_effectively_const();

        Ok(AssignTarget {
            data_type: value_type,
            is_mutable: !is_const,
            kind: AssignTargetKind::IndexSetter {
                setter_hash: overload.func_hash,
                getter_hash,
                index_count: index_types.len() as u8,
            },
        })
    } else {
        // Check for opIndex that returns a reference (lvalue)
        let op_index_methods = {
            let class = compiler
                .ctx()
                .get_type(obj_info.data_type.type_hash)
                .and_then(|e| e.as_class())
                .ok_or_else(|| CompilationError::Internal {
                    message: "type not found".to_string(),
                })?;

            class.find_methods("opIndex").to_vec()
        };

        if !op_index_methods.is_empty() {
            use crate::overload::resolve_overload;

            // Resolve which opIndex overload to use
            let overload = resolve_overload(&op_index_methods, &index_types, compiler.ctx(), span)?;

            // Get return type and check if it's a reference (lvalue)
            let func = compiler
                .ctx()
                .get_function(overload.func_hash)
                .ok_or_else(|| CompilationError::Internal {
                    message: format!("opIndex method not found: {:?}", overload.func_hash),
                })?;

            let return_type = func.def.return_type;

            // opIndex must return a reference for assignment
            // A non-const opIndex returns an lvalue reference
            let is_const_method = func.def.traits.is_const;
            if is_const_method {
                return Err(CompilationError::CannotModifyConst {
                    message: format!("cannot assign via const opIndex on type '{}'", class_name),
                    span,
                });
            }

            let is_const_obj = obj_info.data_type.is_effectively_const();

            Ok(AssignTarget {
                data_type: return_type,
                is_mutable: !is_const_obj,
                kind: AssignTargetKind::IndexRef {
                    method_hash: overload.func_hash,
                    index_count: index_types.len() as u8,
                },
            })
        } else {
            Err(CompilationError::Other {
                message: format!(
                    "type '{}' does not support index assignment (no set_opIndex or opIndex)",
                    class_name
                ),
                span,
            })
        }
    }
}

/// Emit code to load the current value of a target (for compound assignment).
fn emit_load(compiler: &mut ExprCompiler<'_, '_>, target: &AssignTarget, span: Span) -> Result<()> {
    match &target.kind {
        AssignTargetKind::Local { slot } => {
            compiler.emitter().emit_get_local(*slot);
        }
        AssignTargetKind::Global { hash } => {
            compiler.emitter().emit_get_global(*hash);
        }
        AssignTargetKind::Field { field_index } => {
            // Object should already be on stack from analyze phase
            // Dup it so we can use it for both get and set
            compiler.emitter().emit_dup();
            compiler.emitter().emit_get_field(*field_index);
        }
        AssignTargetKind::VirtualProperty { getter_hash, .. } => {
            // For compound assignment, we need to call the getter first
            if let Some(getter) = getter_hash {
                // Dup object so we can use it for both get and set
                compiler.emitter().emit_dup();
                // Call getter (no arguments, just the object as receiver)
                compiler.emitter().emit_call_method(*getter, 0);
            } else {
                return Err(CompilationError::Other {
                    message: "compound assignment on write-only property not supported".to_string(),
                    span,
                });
            }
        }
        AssignTargetKind::IndexSetter {
            getter_hash,
            index_count,
            ..
        } => {
            // For compound assignment on index (set_opIndex style), call the getter
            // Stack before: [object, index0, index1, ...]
            // We need to duplicate all values for getter, keeping originals for setter
            if let Some(getter) = getter_hash {
                // Stack: [obj, idx0, idx1, ..., idx_{n-1}]
                // Use Pick to duplicate each value from bottom to top
                // Pick(n) copies the value at offset n from top
                // For [obj, idx]: pick(1) -> [obj, idx, obj], pick(1) -> [obj, idx, obj, idx]
                let total = *index_count + 1; // object + indices
                for i in (0..total).rev() {
                    // Pick from increasing depth as we add to stack
                    // First pick: offset = total - 1 (bottom element = object)
                    // After first pick, stack is one deeper, so offset increases
                    compiler.emitter().emit_pick(total - 1 + (total - 1 - i));
                }
                // Stack is now: [obj, idx..., obj, idx...]
                // Call getter with indices
                compiler.emitter().emit_call_method(*getter, *index_count);
                // Stack is now: [obj, idx..., current_value]
            } else {
                return Err(CompilationError::Other {
                    message: "compound assignment on index requires get_opIndex".to_string(),
                    span,
                });
            }
        }
        AssignTargetKind::IndexRef {
            method_hash,
            index_count,
        } => {
            // For compound assignment on index (opIndex reference style):
            // opIndex returns a reference - we call it once, dup the ref,
            // deref to get current value. After binary op, store through ref.
            //
            // Stack before: [object, index0, index1, ...]
            // Call opIndex (consumes obj + indices, returns ref)
            compiler
                .emitter()
                .emit_call_method(*method_hash, *index_count);
            // Stack: [ref]
            // Dup ref (need it for store later)
            compiler.emitter().emit_dup();
            // Stack: [ref, ref]
            // Deref to get current value (GetField 0 = load through reference)
            compiler.emitter().emit_get_field(0);
            // Stack: [ref, current_value]
            // After binary op: [ref, new_value]
            // emit_store will: swap, SetField(0), pop
        }
    }
    Ok(())
}

/// Emit code to store a value to a target.
fn emit_store(compiler: &mut ExprCompiler<'_, '_>, target: &AssignTarget) -> Result<()> {
    match &target.kind {
        AssignTargetKind::Local { slot } => {
            compiler.emitter().emit_set_local(*slot);
        }
        AssignTargetKind::Global { hash } => {
            compiler.emitter().emit_set_global(*hash);
        }
        AssignTargetKind::Field { field_index } => {
            compiler.emitter().emit_set_field(*field_index);
        }
        AssignTargetKind::VirtualProperty { setter_hash, .. } => {
            // Call setter method with (object, value) on stack
            // Stack: [object, value] -> call set_propName(value)
            compiler.emitter().emit_call_method(*setter_hash, 1);
        }
        AssignTargetKind::IndexSetter {
            setter_hash,
            index_count,
            ..
        } => {
            // Call set_opIndex with (object, indices..., value) already on stack
            // arg_count is indices + value
            compiler
                .emitter()
                .emit_call_method(*setter_hash, *index_count + 1);
        }
        AssignTargetKind::IndexRef { .. } => {
            // For simple assignment: stack is [ref, value]
            // For compound assignment: stack is [ref, new_value]
            // Swap to get [value, ref], then store through ref
            compiler.emitter().emit(OpCode::Swap);
            // Stack: [value, ref]
            // SetField(0) stores value through reference
            compiler.emitter().emit_set_field(0);
            // Stack: [] (both consumed)
        }
    }
    Ok(())
}

/// Convert a compound assignment operator to its corresponding binary operator.
fn compound_to_binary_op(op: AssignOp, _span: Span) -> Result<BinaryOp> {
    match op {
        AssignOp::AddAssign => Ok(BinaryOp::Add),
        AssignOp::SubAssign => Ok(BinaryOp::Sub),
        AssignOp::MulAssign => Ok(BinaryOp::Mul),
        AssignOp::DivAssign => Ok(BinaryOp::Div),
        AssignOp::ModAssign => Ok(BinaryOp::Mod),
        AssignOp::PowAssign => Ok(BinaryOp::Pow),
        AssignOp::AndAssign => Ok(BinaryOp::BitwiseAnd),
        AssignOp::OrAssign => Ok(BinaryOp::BitwiseOr),
        AssignOp::XorAssign => Ok(BinaryOp::BitwiseXor),
        AssignOp::ShlAssign => Ok(BinaryOp::ShiftLeft),
        AssignOp::ShrAssign => Ok(BinaryOp::ShiftRight),
        AssignOp::UshrAssign => Ok(BinaryOp::ShiftRightUnsigned),
        AssignOp::Assign => Err(CompilationError::Internal {
            message: "simple assignment should not reach compound_to_binary_op".to_string(),
        }),
    }
}

/// Emit the bytecode for a binary operation.
///
/// For compound assignment, the stack has [left_value, right_value].
/// For reverse operators (MethodOnRight), we need to swap so the right
/// operand becomes the receiver.
fn emit_binary_op(
    compiler: &mut ExprCompiler<'_, '_>,
    resolution: &OperatorResolution,
    span: Span,
) -> Result<()> {
    match resolution {
        OperatorResolution::Primitive {
            opcode,
            left_conv,
            right_conv,
            ..
        } => {
            // Stack: [left, right]
            // Apply conversions if needed, then emit opcode
            if let Some(conv) = left_conv {
                // Need to convert left value which is under right on stack
                // Swap, convert, swap back
                compiler.emitter().emit(OpCode::Swap);
                compiler.emitter().emit(*conv);
                compiler.emitter().emit(OpCode::Swap);
            }
            if let Some(conv) = right_conv {
                compiler.emitter().emit(*conv);
            }
            compiler.emitter().emit(*opcode);
        }
        OperatorResolution::MethodOnLeft {
            method_hash,
            arg_conversion,
            ..
        } => {
            // Stack: [left (receiver), right (arg)]
            if let Some(conv) = arg_conversion {
                compiler.emitter().emit(*conv);
            }
            compiler.emitter().emit_call_method(*method_hash, 1);
        }
        OperatorResolution::MethodOnRight {
            method_hash,
            arg_conversion,
            ..
        } => {
            // Stack: [left (arg), right (receiver)]
            // For reverse operator like opAdd_r, right is the receiver
            // Swap so receiver is below arg: [right, left]
            compiler.emitter().emit(OpCode::Swap);
            if let Some(conv) = arg_conversion {
                compiler.emitter().emit(*conv);
            }
            compiler.emitter().emit_call_method(*method_hash, 1);
        }
        OperatorResolution::HandleComparison { .. } => {
            return Err(CompilationError::Other {
                message: "handle comparison not valid for compound assignment".to_string(),
                span,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::ConstantPool;
    use crate::context::CompilationContext;
    use crate::emit::BytecodeEmitter;
    use angelscript_core::entries::{ClassEntry, FunctionEntry, PropertyEntry};
    use angelscript_core::{
        FunctionDef, FunctionTraits, Param, Span, TypeKind, Visibility, primitives,
    };
    use angelscript_parser::ast::{
        Expr, Ident, IdentExpr, IndexExpr, IndexItem, LiteralExpr, LiteralKind, MemberAccess,
        MemberExpr,
    };
    use angelscript_registry::SymbolRegistry;
    use bumpalo::Bump;

    fn create_test_compiler<'a, 'ctx>(
        ctx: &'a mut CompilationContext<'ctx>,
        emitter: &'a mut BytecodeEmitter,
    ) -> ExprCompiler<'a, 'ctx> {
        ExprCompiler::new(ctx, emitter, None)
    }

    fn make_ident_expr<'a>(arena: &'a Bump, name: &'a str) -> &'a Expr<'a> {
        arena.alloc(Expr::Ident(IdentExpr {
            scope: None,
            ident: Ident::new(name, Span::new(1, 1, name.len() as u32)),
            type_args: &[],
            span: Span::new(1, 1, name.len() as u32),
        }))
    }

    fn make_int_literal<'a>(arena: &'a Bump, value: i64) -> &'a Expr<'a> {
        arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(value),
            span: Span::new(1, 1, 1),
        }))
    }

    fn make_member_expr<'a>(arena: &'a Bump, obj: &'a Expr<'a>, field: &'a str) -> &'a Expr<'a> {
        arena.alloc(Expr::Member(arena.alloc(MemberExpr {
            object: obj,
            member: MemberAccess::Field(Ident::new(field, Span::new(1, 1, field.len() as u32))),
            span: Span::new(1, 1, 10),
        })))
    }

    fn make_index_expr<'a>(
        arena: &'a Bump,
        obj: &'a Expr<'a>,
        index: &'a Expr<'a>,
    ) -> &'a Expr<'a> {
        let indices = arena.alloc_slice_copy(&[IndexItem {
            name: None,
            index,
            span: Span::new(1, 1, 1),
        }]);
        arena.alloc(Expr::Index(arena.alloc(IndexExpr {
            object: obj,
            indices,
            span: Span::new(1, 1, 10),
        })))
    }

    /// Create a class with a read-write virtual property (has both getter and setter).
    fn create_class_with_rw_property(
        registry: &mut SymbolRegistry,
    ) -> (TypeHash, TypeHash, TypeHash) {
        let type_hash = TypeHash::from_name("Container");
        let getter_hash = TypeHash::from_method(type_hash, "get_value", &[]);
        let setter_hash = TypeHash::from_method(type_hash, "set_value", &[primitives::INT32]);

        // Create class with virtual property
        let mut class = ClassEntry::ffi("Container", TypeKind::script_object());
        class.properties.push(PropertyEntry::read_write(
            "value",
            DataType::simple(primitives::INT32),
            getter_hash,
            setter_hash,
        ));
        class.add_method("get_value", getter_hash);
        class.add_method("set_value", setter_hash);
        registry.register_type(class.into()).unwrap();

        // Register getter method
        let mut getter_traits = FunctionTraits::default();
        getter_traits.is_const = true;
        let getter_def = FunctionDef::new(
            getter_hash,
            "get_value".to_string(),
            vec![],
            vec![],
            DataType::simple(primitives::INT32),
            Some(type_hash),
            getter_traits,
            true,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(getter_def))
            .unwrap();

        // Register setter method
        let setter_traits = FunctionTraits::default();
        let setter_def = FunctionDef::new(
            setter_hash,
            "set_value".to_string(),
            vec![],
            vec![Param {
                name: "val".to_string(),
                data_type: DataType::simple(primitives::INT32),
                has_default: false,
                if_handle_then_const: false,
            }],
            DataType::simple(primitives::VOID),
            Some(type_hash),
            setter_traits,
            true,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(setter_def))
            .unwrap();

        (type_hash, getter_hash, setter_hash)
    }

    /// Create a class with a read-only virtual property (has getter but no setter).
    fn create_class_with_ro_property(registry: &mut SymbolRegistry) -> (TypeHash, TypeHash) {
        let type_hash = TypeHash::from_name("ReadOnlyContainer");
        let getter_hash = TypeHash::from_method(type_hash, "get_length", &[]);

        let mut class = ClassEntry::ffi("ReadOnlyContainer", TypeKind::script_object());
        class.properties.push(PropertyEntry::read_only(
            "length",
            DataType::simple(primitives::INT32),
            getter_hash,
        ));
        class.add_method("get_length", getter_hash);
        registry.register_type(class.into()).unwrap();

        // Register getter method
        let mut getter_traits = FunctionTraits::default();
        getter_traits.is_const = true;
        let getter_def = FunctionDef::new(
            getter_hash,
            "get_length".to_string(),
            vec![],
            vec![],
            DataType::simple(primitives::INT32),
            Some(type_hash),
            getter_traits,
            true,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(getter_def))
            .unwrap();

        (type_hash, getter_hash)
    }

    /// Create a class with a write-only virtual property (has setter but no getter).
    fn create_class_with_wo_property(registry: &mut SymbolRegistry) -> (TypeHash, TypeHash) {
        let type_hash = TypeHash::from_name("WriteOnlyContainer");
        let setter_hash = TypeHash::from_method(type_hash, "set_secret", &[primitives::INT32]);

        let mut class = ClassEntry::ffi("WriteOnlyContainer", TypeKind::script_object());
        class.properties.push(PropertyEntry::new(
            "secret",
            DataType::simple(primitives::INT32),
            Visibility::Public,
            None, // no getter
            Some(setter_hash),
        ));
        class.add_method("set_secret", setter_hash);
        registry.register_type(class.into()).unwrap();

        // Register setter method
        let setter_traits = FunctionTraits::default();
        let setter_def = FunctionDef::new(
            setter_hash,
            "set_secret".to_string(),
            vec![],
            vec![Param {
                name: "val".to_string(),
                data_type: DataType::simple(primitives::INT32),
                has_default: false,
                if_handle_then_const: false,
            }],
            DataType::simple(primitives::VOID),
            Some(type_hash),
            setter_traits,
            true,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(setter_def))
            .unwrap();

        (type_hash, setter_hash)
    }

    /// Create a class with set_opIndex/get_opIndex (property accessor style).
    fn create_class_with_index_setter(
        registry: &mut SymbolRegistry,
    ) -> (TypeHash, TypeHash, TypeHash) {
        let type_hash = TypeHash::from_name("IndexedContainer");
        let getter_hash = TypeHash::from_method(type_hash, "get_opIndex", &[primitives::INT32]);
        let setter_hash = TypeHash::from_method(
            type_hash,
            "set_opIndex",
            &[primitives::INT32, primitives::INT32],
        );

        let mut class = ClassEntry::ffi("IndexedContainer", TypeKind::script_object());
        class.add_method("get_opIndex", getter_hash);
        class.add_method("set_opIndex", setter_hash);
        registry.register_type(class.into()).unwrap();

        // Register get_opIndex method
        let mut getter_traits = FunctionTraits::default();
        getter_traits.is_const = true;
        let getter_def = FunctionDef::new(
            getter_hash,
            "get_opIndex".to_string(),
            vec![],
            vec![Param {
                name: "index".to_string(),
                data_type: DataType::simple(primitives::INT32),
                has_default: false,
                if_handle_then_const: false,
            }],
            DataType::simple(primitives::INT32),
            Some(type_hash),
            getter_traits,
            true,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(getter_def))
            .unwrap();

        // Register set_opIndex method
        let setter_traits = FunctionTraits::default();
        let setter_def = FunctionDef::new(
            setter_hash,
            "set_opIndex".to_string(),
            vec![],
            vec![
                Param {
                    name: "index".to_string(),
                    data_type: DataType::simple(primitives::INT32),
                    has_default: false,
                    if_handle_then_const: false,
                },
                Param {
                    name: "value".to_string(),
                    data_type: DataType::simple(primitives::INT32),
                    has_default: false,
                    if_handle_then_const: false,
                },
            ],
            DataType::simple(primitives::VOID),
            Some(type_hash),
            setter_traits,
            true,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(setter_def))
            .unwrap();

        (type_hash, getter_hash, setter_hash)
    }

    /// Create a class with opIndex returning reference (direct style, non-const).
    fn create_class_with_opindex_ref(registry: &mut SymbolRegistry) -> (TypeHash, TypeHash) {
        let type_hash = TypeHash::from_name("RefIndexedContainer");
        let opindex_hash = TypeHash::from_method(type_hash, "opIndex", &[primitives::INT32]);

        let mut class = ClassEntry::ffi("RefIndexedContainer", TypeKind::script_object());
        class.add_method("opIndex", opindex_hash);
        registry.register_type(class.into()).unwrap();

        // Register opIndex method (non-const, returns reference)
        let opindex_traits = FunctionTraits::default(); // NOT const
        let opindex_def = FunctionDef::new(
            opindex_hash,
            "opIndex".to_string(),
            vec![],
            vec![Param {
                name: "index".to_string(),
                data_type: DataType::simple(primitives::INT32),
                has_default: false,
                if_handle_then_const: false,
            }],
            DataType::simple(primitives::INT32), // Returns int reference
            Some(type_hash),
            opindex_traits,
            true,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(opindex_def))
            .unwrap();

        (type_hash, opindex_hash)
    }

    /// Create a class with const opIndex (read-only, cannot be assigned to).
    fn create_class_with_const_opindex(registry: &mut SymbolRegistry) -> (TypeHash, TypeHash) {
        let type_hash = TypeHash::from_name("ConstIndexedContainer");
        let opindex_hash = TypeHash::from_method(type_hash, "opIndex", &[primitives::INT32]);

        let mut class = ClassEntry::ffi("ConstIndexedContainer", TypeKind::script_object());
        class.add_method("opIndex", opindex_hash);
        registry.register_type(class.into()).unwrap();

        // Register opIndex method (CONST - read-only)
        let mut opindex_traits = FunctionTraits::default();
        opindex_traits.is_const = true;
        let opindex_def = FunctionDef::new(
            opindex_hash,
            "opIndex".to_string(),
            vec![],
            vec![Param {
                name: "index".to_string(),
                data_type: DataType::simple(primitives::INT32),
                has_default: false,
                if_handle_then_const: false,
            }],
            DataType::simple(primitives::INT32),
            Some(type_hash),
            opindex_traits,
            true,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(opindex_def))
            .unwrap();

        (type_hash, opindex_hash)
    }

    #[test]
    fn simple_assignment_to_local() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Declare a local variable
        let _ = ctx.declare_local(
            "x".to_string(),
            DataType::simple(primitives::INT32),
            false,
            Span::default(),
        );

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();
        let target = make_ident_expr(&arena, "x");
        let value = make_int_literal(&arena, 42);

        let assign_expr = arena.alloc(AssignExpr {
            target,
            op: AssignOp::Assign,
            value,
            span: Span::new(1, 1, 6),
        });

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_assign(&mut compiler, assign_expr);

        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::INT32);
        assert!(!info.is_lvalue); // Assignment returns rvalue

        let chunk = emitter.finish_chunk();
        // Bytecode: Constant, SetLocal
        chunk.assert_opcodes(&[OpCode::Constant, OpCode::SetLocal]);
    }

    #[test]
    fn assignment_to_const_rejected() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Declare a const local variable
        let _ = ctx.declare_local(
            "x".to_string(),
            DataType::simple(primitives::INT32),
            true, // const
            Span::default(),
        );

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();
        let target = make_ident_expr(&arena, "x");
        let value = make_int_literal(&arena, 42);

        let assign_expr = arena.alloc(AssignExpr {
            target,
            op: AssignOp::Assign,
            value,
            span: Span::new(1, 1, 6),
        });

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_assign(&mut compiler, assign_expr);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CompilationError::CannotModifyConst { .. }
        ));
    }

    #[test]
    fn assignment_to_rvalue_rejected() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();
        let target = make_int_literal(&arena, 5); // rvalue - can't assign to this
        let value = make_int_literal(&arena, 42);

        let assign_expr = arena.alloc(AssignExpr {
            target,
            op: AssignOp::Assign,
            value,
            span: Span::new(1, 1, 6),
        });

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_assign(&mut compiler, assign_expr);

        assert!(result.is_err());
        // Should fail because a literal is not a valid assignment target
        match result.unwrap_err() {
            CompilationError::Other { message, .. } => {
                assert!(
                    message.contains("invalid assignment target"),
                    "Expected error about invalid assignment target, got: {}",
                    message
                );
            }
            other => panic!("Expected Other error, got: {:?}", other),
        }
    }

    #[test]
    fn compound_assignment_add() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Declare a local variable
        let _ = ctx.declare_local(
            "x".to_string(),
            DataType::simple(primitives::INT32),
            false,
            Span::default(),
        );

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();
        let target = make_ident_expr(&arena, "x");
        let value = make_int_literal(&arena, 5);

        let assign_expr = arena.alloc(AssignExpr {
            target,
            op: AssignOp::AddAssign,
            value,
            span: Span::new(1, 1, 6),
        });

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_assign(&mut compiler, assign_expr);

        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::INT32);

        let chunk = emitter.finish_chunk();
        // Bytecode: GetLocal, Constant, AddI32, SetLocal
        chunk.assert_opcodes(&[
            OpCode::GetLocal,
            OpCode::Constant,
            OpCode::AddI32,
            OpCode::SetLocal,
        ]);
    }

    #[test]
    fn compound_to_binary_op_mapping() {
        let span = Span::default();

        assert_eq!(
            compound_to_binary_op(AssignOp::AddAssign, span).unwrap(),
            BinaryOp::Add
        );
        assert_eq!(
            compound_to_binary_op(AssignOp::SubAssign, span).unwrap(),
            BinaryOp::Sub
        );
        assert_eq!(
            compound_to_binary_op(AssignOp::MulAssign, span).unwrap(),
            BinaryOp::Mul
        );
        assert_eq!(
            compound_to_binary_op(AssignOp::DivAssign, span).unwrap(),
            BinaryOp::Div
        );
        assert_eq!(
            compound_to_binary_op(AssignOp::ModAssign, span).unwrap(),
            BinaryOp::Mod
        );
        assert_eq!(
            compound_to_binary_op(AssignOp::PowAssign, span).unwrap(),
            BinaryOp::Pow
        );
        assert_eq!(
            compound_to_binary_op(AssignOp::AndAssign, span).unwrap(),
            BinaryOp::BitwiseAnd
        );
        assert_eq!(
            compound_to_binary_op(AssignOp::OrAssign, span).unwrap(),
            BinaryOp::BitwiseOr
        );
        assert_eq!(
            compound_to_binary_op(AssignOp::XorAssign, span).unwrap(),
            BinaryOp::BitwiseXor
        );
        assert_eq!(
            compound_to_binary_op(AssignOp::ShlAssign, span).unwrap(),
            BinaryOp::ShiftLeft
        );
        assert_eq!(
            compound_to_binary_op(AssignOp::ShrAssign, span).unwrap(),
            BinaryOp::ShiftRight
        );
        assert_eq!(
            compound_to_binary_op(AssignOp::UshrAssign, span).unwrap(),
            BinaryOp::ShiftRightUnsigned
        );
    }

    #[test]
    fn assignment_to_this_rejected() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();
        let target = make_ident_expr(&arena, "this");
        let value = make_int_literal(&arena, 42);

        let assign_expr = arena.alloc(AssignExpr {
            target,
            op: AssignOp::Assign,
            value,
            span: Span::new(1, 1, 10),
        });

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_assign(&mut compiler, assign_expr);

        assert!(result.is_err());
        match result.unwrap_err() {
            CompilationError::Other { message, .. } => {
                assert!(
                    message.contains("cannot assign to 'this'"),
                    "Expected error about assigning to 'this', got: {}",
                    message
                );
            }
            other => panic!("Expected Other error, got: {:?}", other),
        }
    }

    #[test]
    fn assignment_to_undefined_variable_rejected() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();
        let target = make_ident_expr(&arena, "undefined_var");
        let value = make_int_literal(&arena, 42);

        let assign_expr = arena.alloc(AssignExpr {
            target,
            op: AssignOp::Assign,
            value,
            span: Span::new(1, 1, 20),
        });

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_assign(&mut compiler, assign_expr);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CompilationError::UndefinedVariable { .. }
        ));
    }

    // =========================================================================
    // Virtual property assignment tests
    // =========================================================================

    #[test]
    fn virtual_property_simple_assignment() {
        let mut registry = SymbolRegistry::with_primitives();
        let (type_hash, _getter_hash, _setter_hash) = create_class_with_rw_property(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Declare a local variable of the Container type
        let _ = ctx.declare_local(
            "container".to_string(),
            DataType::simple(type_hash),
            false,
            Span::default(),
        );

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();
        let obj = make_ident_expr(&arena, "container");
        let target = make_member_expr(&arena, obj, "value");
        let value = make_int_literal(&arena, 42);

        let assign_expr = arena.alloc(AssignExpr {
            target,
            op: AssignOp::Assign,
            value,
            span: Span::new(1, 1, 20),
        });

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_assign(&mut compiler, assign_expr);

        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::INT32);

        let chunk = emitter.finish_chunk();
        // Bytecode: GetLocal container, Constant 42, CallMethod setter
        chunk.assert_opcodes(&[OpCode::GetLocal, OpCode::Constant, OpCode::CallMethod]);
    }

    #[test]
    fn virtual_property_compound_assignment() {
        let mut registry = SymbolRegistry::with_primitives();
        let (type_hash, _getter_hash, _setter_hash) = create_class_with_rw_property(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Declare a local variable of the Container type
        let _ = ctx.declare_local(
            "container".to_string(),
            DataType::simple(type_hash),
            false,
            Span::default(),
        );

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();
        let obj = make_ident_expr(&arena, "container");
        let target = make_member_expr(&arena, obj, "value");
        let value = make_int_literal(&arena, 5);

        let assign_expr = arena.alloc(AssignExpr {
            target,
            op: AssignOp::AddAssign, // container.value += 5
            value,
            span: Span::new(1, 1, 20),
        });

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_assign(&mut compiler, assign_expr);

        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::INT32);

        let chunk = emitter.finish_chunk();
        // Bytecode: GetLocal, Dup, CallMethod getter, Constant, AddI32, CallMethod setter
        chunk.assert_opcodes(&[
            OpCode::GetLocal,
            OpCode::Dup,
            OpCode::CallMethod, // getter
            OpCode::Constant,
            OpCode::AddI32,
            OpCode::CallMethod, // setter
        ]);
    }

    #[test]
    fn virtual_property_read_only_assignment_rejected() {
        let mut registry = SymbolRegistry::with_primitives();
        let (type_hash, _getter_hash) = create_class_with_ro_property(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Declare a local variable
        let _ = ctx.declare_local(
            "container".to_string(),
            DataType::simple(type_hash),
            false,
            Span::default(),
        );

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();
        let obj = make_ident_expr(&arena, "container");
        let target = make_member_expr(&arena, obj, "length");
        let value = make_int_literal(&arena, 10);

        let assign_expr = arena.alloc(AssignExpr {
            target,
            op: AssignOp::Assign,
            value,
            span: Span::new(1, 1, 20),
        });

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_assign(&mut compiler, assign_expr);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CompilationError::CannotModifyConst { .. }
        ));
    }

    #[test]
    fn virtual_property_write_only_simple_assignment() {
        let mut registry = SymbolRegistry::with_primitives();
        let (type_hash, _setter_hash) = create_class_with_wo_property(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Declare a local variable
        let _ = ctx.declare_local(
            "container".to_string(),
            DataType::simple(type_hash),
            false,
            Span::default(),
        );

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();
        let obj = make_ident_expr(&arena, "container");
        let target = make_member_expr(&arena, obj, "secret");
        let value = make_int_literal(&arena, 123);

        let assign_expr = arena.alloc(AssignExpr {
            target,
            op: AssignOp::Assign,
            value,
            span: Span::new(1, 1, 20),
        });

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_assign(&mut compiler, assign_expr);

        // Write-only properties can be assigned to
        assert!(result.is_ok());

        let chunk = emitter.finish_chunk();
        // Bytecode: GetLocal container, Constant value, CallMethod setter
        chunk.assert_opcodes(&[OpCode::GetLocal, OpCode::Constant, OpCode::CallMethod]);
    }

    #[test]
    fn virtual_property_write_only_compound_assignment_rejected() {
        let mut registry = SymbolRegistry::with_primitives();
        let (type_hash, _setter_hash) = create_class_with_wo_property(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Declare a local variable
        let _ = ctx.declare_local(
            "container".to_string(),
            DataType::simple(type_hash),
            false,
            Span::default(),
        );

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();
        let obj = make_ident_expr(&arena, "container");
        let target = make_member_expr(&arena, obj, "secret");
        let value = make_int_literal(&arena, 5);

        let assign_expr = arena.alloc(AssignExpr {
            target,
            op: AssignOp::AddAssign, // compound assignment requires getter
            value,
            span: Span::new(1, 1, 20),
        });

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_assign(&mut compiler, assign_expr);

        // Compound assignment on write-only property should fail (no getter)
        assert!(result.is_err());
        match result.unwrap_err() {
            CompilationError::Other { message, .. } => {
                assert!(
                    message.contains("compound assignment on write-only property"),
                    "Expected error about compound assignment on write-only property, got: {}",
                    message
                );
            }
            other => panic!("Expected Other error, got: {:?}", other),
        }
    }

    // =========================================================================
    // Index assignment tests (set_opIndex/get_opIndex style)
    // =========================================================================

    #[test]
    fn index_setter_simple_assignment() {
        let mut registry = SymbolRegistry::with_primitives();
        let (type_hash, _getter_hash, _setter_hash) = create_class_with_index_setter(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        let _ = ctx.declare_local(
            "container".to_string(),
            DataType::simple(type_hash),
            false,
            Span::default(),
        );

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();
        let obj = make_ident_expr(&arena, "container");
        let index = make_int_literal(&arena, 0);
        let target = make_index_expr(&arena, obj, index);
        let value = make_int_literal(&arena, 42);

        let assign_expr = arena.alloc(AssignExpr {
            target,
            op: AssignOp::Assign,
            value,
            span: Span::new(1, 1, 20),
        });

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_assign(&mut compiler, assign_expr);

        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::INT32);

        let chunk = emitter.finish_chunk();
        // Bytecode: GetLocal container, PushZero index, Constant value, CallMethod setter
        chunk.assert_opcodes(&[
            OpCode::GetLocal,
            OpCode::PushZero,
            OpCode::Constant,
            OpCode::CallMethod,
        ]);
    }

    #[test]
    fn index_setter_compound_assignment() {
        let mut registry = SymbolRegistry::with_primitives();
        let (type_hash, _getter_hash, _setter_hash) = create_class_with_index_setter(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        let _ = ctx.declare_local(
            "container".to_string(),
            DataType::simple(type_hash),
            false,
            Span::default(),
        );

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();
        let obj = make_ident_expr(&arena, "container");
        let index = make_int_literal(&arena, 0);
        let target = make_index_expr(&arena, obj, index);
        let value = make_int_literal(&arena, 5);

        let assign_expr = arena.alloc(AssignExpr {
            target,
            op: AssignOp::AddAssign, // container[0] += 5
            value,
            span: Span::new(1, 1, 20),
        });

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_assign(&mut compiler, assign_expr);

        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::INT32);

        let chunk = emitter.finish_chunk();
        // Bytecode: GetLocal, PushZero, Pick, Pick, CallMethod getter, Constant, AddI32, CallMethod setter
        chunk.assert_opcodes(&[
            OpCode::GetLocal,
            OpCode::PushZero,
            OpCode::Pick,
            OpCode::Pick,
            OpCode::CallMethod, // getter
            OpCode::Constant,
            OpCode::AddI32,
            OpCode::CallMethod, // setter
        ]);
    }

    // =========================================================================
    // Index assignment tests (opIndex reference style)
    // =========================================================================

    #[test]
    fn index_ref_simple_assignment() {
        let mut registry = SymbolRegistry::with_primitives();
        let (type_hash, _opindex_hash) = create_class_with_opindex_ref(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        let _ = ctx.declare_local(
            "container".to_string(),
            DataType::simple(type_hash),
            false,
            Span::default(),
        );

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();
        let obj = make_ident_expr(&arena, "container");
        let index = make_int_literal(&arena, 0);
        let target = make_index_expr(&arena, obj, index);
        let value = make_int_literal(&arena, 42);

        let assign_expr = arena.alloc(AssignExpr {
            target,
            op: AssignOp::Assign,
            value,
            span: Span::new(1, 1, 20),
        });

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_assign(&mut compiler, assign_expr);

        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::INT32);

        let chunk = emitter.finish_chunk();
        // Bytecode: GetLocal, PushZero, CallMethod opIndex (returns ref), Constant, Swap, SetField
        chunk.assert_opcodes(&[
            OpCode::GetLocal,
            OpCode::PushZero,
            OpCode::CallMethod,
            OpCode::Constant,
            OpCode::Swap,
            OpCode::SetField,
        ]);
    }

    #[test]
    fn index_ref_compound_assignment() {
        let mut registry = SymbolRegistry::with_primitives();
        let (type_hash, _opindex_hash) = create_class_with_opindex_ref(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        let _ = ctx.declare_local(
            "container".to_string(),
            DataType::simple(type_hash),
            false,
            Span::default(),
        );

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();
        let obj = make_ident_expr(&arena, "container");
        let index = make_int_literal(&arena, 0);
        let target = make_index_expr(&arena, obj, index);
        let value = make_int_literal(&arena, 5);

        let assign_expr = arena.alloc(AssignExpr {
            target,
            op: AssignOp::AddAssign, // container[0] += 5
            value,
            span: Span::new(1, 1, 20),
        });

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_assign(&mut compiler, assign_expr);

        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::INT32);

        let chunk = emitter.finish_chunk();
        // Bytecode: GetLocal, PushZero, CallMethod opIndex, Dup, GetField, Constant, AddI32, Swap, SetField
        chunk.assert_opcodes(&[
            OpCode::GetLocal,
            OpCode::PushZero,
            OpCode::CallMethod,
            OpCode::Dup,
            OpCode::GetField,
            OpCode::Constant,
            OpCode::AddI32,
            OpCode::Swap,
            OpCode::SetField,
        ]);
    }

    #[test]
    fn const_opindex_assignment_rejected() {
        let mut registry = SymbolRegistry::with_primitives();
        let (type_hash, _opindex_hash) = create_class_with_const_opindex(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        let _ = ctx.declare_local(
            "container".to_string(),
            DataType::simple(type_hash),
            false,
            Span::default(),
        );

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();
        let obj = make_ident_expr(&arena, "container");
        let index = make_int_literal(&arena, 0);
        let target = make_index_expr(&arena, obj, index);
        let value = make_int_literal(&arena, 42);

        let assign_expr = arena.alloc(AssignExpr {
            target,
            op: AssignOp::Assign,
            value,
            span: Span::new(1, 1, 20),
        });

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_assign(&mut compiler, assign_expr);

        // Const opIndex cannot be assigned to
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CompilationError::CannotModifyConst { .. }
        ));
    }

    // =========================================================================
    // Method call RHS tests
    // =========================================================================

    /// Create a class with a method that returns int.
    fn create_class_with_get_value(registry: &mut SymbolRegistry) -> (TypeHash, TypeHash) {
        let type_hash = TypeHash::from_name("Provider");
        let method_hash = TypeHash::from_method(type_hash, "getValue", &[]);

        let mut class = ClassEntry::ffi("Provider", TypeKind::script_object());
        class.add_method("getValue", method_hash);
        registry.register_type(class.into()).unwrap();

        let method_traits = FunctionTraits::default();
        let method_def = FunctionDef::new(
            method_hash,
            "getValue".to_string(),
            vec![],
            vec![],
            DataType::simple(primitives::INT32),
            Some(type_hash),
            method_traits,
            true,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(method_def))
            .unwrap();

        (type_hash, method_hash)
    }

    fn make_method_call_expr<'a>(
        arena: &'a Bump,
        obj: &'a Expr<'a>,
        method: &'a str,
    ) -> &'a Expr<'a> {
        use angelscript_parser::ast::MemberAccess;

        // Method calls are represented as MemberExpr with MemberAccess::Method
        arena.alloc(Expr::Member(arena.alloc(MemberExpr {
            object: obj,
            member: MemberAccess::Method {
                name: Ident::new(method, Span::new(1, 1, method.len() as u32)),
                args: &[],
            },
            span: Span::new(1, 1, 15),
        })))
    }

    #[test]
    fn simple_assignment_method_call_rhs() {
        let mut registry = SymbolRegistry::with_primitives();
        let (provider_type, _method_hash) = create_class_with_get_value(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        let _ = ctx.declare_local(
            "x".to_string(),
            DataType::simple(primitives::INT32),
            false,
            Span::default(),
        );
        let _ = ctx.declare_local(
            "provider".to_string(),
            DataType::simple(provider_type),
            false,
            Span::default(),
        );

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();
        let target = make_ident_expr(&arena, "x");
        let provider = make_ident_expr(&arena, "provider");
        let value = make_method_call_expr(&arena, provider, "getValue");

        let assign_expr = arena.alloc(AssignExpr {
            target,
            op: AssignOp::Assign,
            value,
            span: Span::new(1, 1, 25),
        });

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_assign(&mut compiler, assign_expr);

        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::INT32);

        let chunk = emitter.finish_chunk();
        // Bytecode: GetLocal provider, CallMethod getValue, SetLocal x
        chunk.assert_opcodes(&[OpCode::GetLocal, OpCode::CallMethod, OpCode::SetLocal]);
    }

    #[test]
    fn index_setter_method_call_rhs() {
        let mut registry = SymbolRegistry::with_primitives();
        let (container_type, _getter_hash, _setter_hash) =
            create_class_with_index_setter(&mut registry);
        let (provider_type, _method_hash) = create_class_with_get_value(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        let _ = ctx.declare_local(
            "container".to_string(),
            DataType::simple(container_type),
            false,
            Span::default(),
        );
        let _ = ctx.declare_local(
            "provider".to_string(),
            DataType::simple(provider_type),
            false,
            Span::default(),
        );

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();
        let container = make_ident_expr(&arena, "container");
        let index = make_int_literal(&arena, 0);
        let target = make_index_expr(&arena, container, index);
        let provider = make_ident_expr(&arena, "provider");
        let value = make_method_call_expr(&arena, provider, "getValue");

        let assign_expr = arena.alloc(AssignExpr {
            target,
            op: AssignOp::Assign,
            value,
            span: Span::new(1, 1, 30),
        });

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_assign(&mut compiler, assign_expr);

        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::INT32);

        let chunk = emitter.finish_chunk();
        // Bytecode: GetLocal container, PushZero, GetLocal provider, CallMethod getValue, CallMethod set_opIndex
        chunk.assert_opcodes(&[
            OpCode::GetLocal,
            OpCode::PushZero,
            OpCode::GetLocal,
            OpCode::CallMethod, // getValue
            OpCode::CallMethod, // set_opIndex
        ]);
    }

    #[test]
    fn virtual_property_method_call_rhs() {
        let mut registry = SymbolRegistry::with_primitives();
        let (container_type, _getter_hash, _setter_hash) =
            create_class_with_rw_property(&mut registry);
        let (provider_type, _method_hash) = create_class_with_get_value(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        let _ = ctx.declare_local(
            "container".to_string(),
            DataType::simple(container_type),
            false,
            Span::default(),
        );
        let _ = ctx.declare_local(
            "provider".to_string(),
            DataType::simple(provider_type),
            false,
            Span::default(),
        );

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();
        let container = make_ident_expr(&arena, "container");
        let target = make_member_expr(&arena, container, "value");
        let provider = make_ident_expr(&arena, "provider");
        let value = make_method_call_expr(&arena, provider, "getValue");

        let assign_expr = arena.alloc(AssignExpr {
            target,
            op: AssignOp::Assign,
            value,
            span: Span::new(1, 1, 35),
        });

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_assign(&mut compiler, assign_expr);

        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::INT32);

        let chunk = emitter.finish_chunk();
        // Bytecode: GetLocal container, GetLocal provider, CallMethod getValue, CallMethod set_value
        chunk.assert_opcodes(&[
            OpCode::GetLocal,
            OpCode::GetLocal,
            OpCode::CallMethod, // getValue
            OpCode::CallMethod, // set_value
        ]);
    }

    #[test]
    fn index_ref_method_call_rhs() {
        let mut registry = SymbolRegistry::with_primitives();
        let (container_type, _opindex_hash) = create_class_with_opindex_ref(&mut registry);
        let (provider_type, _method_hash) = create_class_with_get_value(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        let _ = ctx.declare_local(
            "container".to_string(),
            DataType::simple(container_type),
            false,
            Span::default(),
        );
        let _ = ctx.declare_local(
            "provider".to_string(),
            DataType::simple(provider_type),
            false,
            Span::default(),
        );

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();
        let container = make_ident_expr(&arena, "container");
        let index = make_int_literal(&arena, 0);
        let target = make_index_expr(&arena, container, index);
        let provider = make_ident_expr(&arena, "provider");
        let value = make_method_call_expr(&arena, provider, "getValue");

        let assign_expr = arena.alloc(AssignExpr {
            target,
            op: AssignOp::Assign,
            value,
            span: Span::new(1, 1, 30),
        });

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_assign(&mut compiler, assign_expr);

        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::INT32);

        let chunk = emitter.finish_chunk();
        // Bytecode: GetLocal container, PushZero, CallMethod opIndex, GetLocal provider, CallMethod getValue, Swap, SetField
        chunk.assert_opcodes(&[
            OpCode::GetLocal,
            OpCode::PushZero,
            OpCode::CallMethod, // opIndex
            OpCode::GetLocal,
            OpCode::CallMethod, // getValue
            OpCode::Swap,
            OpCode::SetField,
        ]);
    }

    // =========================================================================
    // Implicit this.field assignment tests
    // =========================================================================

    fn create_class_with_field(registry: &mut SymbolRegistry) -> TypeHash {
        let class_hash = TypeHash::from_name("TestClass");

        let mut class = ClassEntry::ffi("TestClass", TypeKind::script_object());
        class.properties.push(PropertyEntry::field(
            "x",
            DataType::simple(primitives::INT32),
            Visibility::Public,
        ));
        class.properties.push(PropertyEntry::field(
            "y",
            DataType::simple(primitives::INT32),
            Visibility::Public,
        ));
        registry.register_type(class.into()).unwrap();

        class_hash
    }

    fn create_test_compiler_with_class<'a, 'ctx>(
        ctx: &'a mut CompilationContext<'ctx>,
        emitter: &'a mut BytecodeEmitter,
        current_class: TypeHash,
    ) -> ExprCompiler<'a, 'ctx> {
        ExprCompiler::new(ctx, emitter, Some(current_class))
    }

    #[test]
    fn implicit_field_simple_assignment() {
        let mut registry = SymbolRegistry::with_primitives();
        let class_hash = create_class_with_field(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Declare 'this' parameter (as methods do)
        let _ = ctx.declare_local(
            "this".to_string(),
            DataType::with_handle(class_hash, false),
            false, // mutable this
            Span::default(),
        );

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();
        let target = make_ident_expr(&arena, "x");
        let value = make_int_literal(&arena, 42);

        let assign_expr = arena.alloc(AssignExpr {
            target,
            op: AssignOp::Assign,
            value,
            span: Span::new(1, 1, 6),
        });

        let mut compiler = create_test_compiler_with_class(&mut ctx, &mut emitter, class_hash);
        let result = compile_assign(&mut compiler, assign_expr);

        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::INT32);

        let chunk = emitter.finish_chunk();
        // Bytecode: GetThis, Constant 42, SetField
        chunk.assert_opcodes(&[OpCode::GetThis, OpCode::Constant, OpCode::SetField]);
    }

    #[test]
    fn implicit_field_compound_assignment() {
        let mut registry = SymbolRegistry::with_primitives();
        let class_hash = create_class_with_field(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Declare 'this' parameter (as methods do)
        let _ = ctx.declare_local(
            "this".to_string(),
            DataType::with_handle(class_hash, false),
            false, // mutable this
            Span::default(),
        );

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();
        let target = make_ident_expr(&arena, "x");
        let value = make_int_literal(&arena, 5);

        let assign_expr = arena.alloc(AssignExpr {
            target,
            op: AssignOp::AddAssign, // x += 5
            value,
            span: Span::new(1, 1, 6),
        });

        let mut compiler = create_test_compiler_with_class(&mut ctx, &mut emitter, class_hash);
        let result = compile_assign(&mut compiler, assign_expr);

        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::INT32);

        let chunk = emitter.finish_chunk();
        // Bytecode: GetThis, Dup, GetField, Constant, AddI32, SetField
        chunk.assert_opcodes(&[
            OpCode::GetThis,
            OpCode::Dup,
            OpCode::GetField,
            OpCode::Constant,
            OpCode::AddI32,
            OpCode::SetField,
        ]);
    }

    #[test]
    fn implicit_field_assignment_const_method_rejected() {
        let mut registry = SymbolRegistry::with_primitives();
        let class_hash = create_class_with_field(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Declare 'this' parameter as const (const method)
        let _ = ctx.declare_local(
            "this".to_string(),
            DataType::with_handle(class_hash, false),
            true, // const this
            Span::default(),
        );

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();
        let target = make_ident_expr(&arena, "x");
        let value = make_int_literal(&arena, 42);

        let assign_expr = arena.alloc(AssignExpr {
            target,
            op: AssignOp::Assign,
            value,
            span: Span::new(1, 1, 6),
        });

        let mut compiler = create_test_compiler_with_class(&mut ctx, &mut emitter, class_hash);
        let result = compile_assign(&mut compiler, assign_expr);

        // Should fail because we're in a const method
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CompilationError::CannotModifyConst { .. }
        ));
    }

    #[test]
    fn implicit_field_local_takes_precedence_in_assignment() {
        let mut registry = SymbolRegistry::with_primitives();
        let class_hash = create_class_with_field(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Declare 'this' parameter
        let _ = ctx.declare_local(
            "this".to_string(),
            DataType::with_handle(class_hash, false),
            false,
            Span::default(),
        );

        // Also declare a local variable named 'x' that shadows the field
        let _ = ctx.declare_local(
            "x".to_string(),
            DataType::simple(primitives::FLOAT),
            false,
            Span::default(),
        );

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();
        let target = make_ident_expr(&arena, "x");
        let value = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Float(42.0),
            span: Span::new(1, 1, 4),
        }));

        let assign_expr = arena.alloc(AssignExpr {
            target,
            op: AssignOp::Assign,
            value,
            span: Span::new(1, 1, 10),
        });

        let mut compiler = create_test_compiler_with_class(&mut ctx, &mut emitter, class_hash);
        let result = compile_assign(&mut compiler, assign_expr);

        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
        let info = result.unwrap();
        // Should be float (local), not int (field)
        assert_eq!(info.data_type.type_hash, primitives::FLOAT);

        let chunk = emitter.finish_chunk();
        // Should use Constant + SetLocal, not GetThis + Constant + SetField
        chunk.assert_opcodes(&[OpCode::Constant, OpCode::SetLocal]);
    }
}
