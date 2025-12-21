//! Member access and index expression compilation.
//!
//! This module handles:
//! - Field access: `obj.field`
//! - Property access: `obj.property` (via getter/setter)
//! - Method calls: `obj.method(args)`
//! - Index access: `arr[i]` (via opIndex/get_opIndex/set_opIndex)

use angelscript_core::{CompilationError, DataType, OperatorBehavior, Span, TypeHash};
use angelscript_parser::ast::{IndexExpr, MemberAccess, MemberExpr};

use crate::expr_info::ExprInfo;

use super::ExprCompiler;
use super::calls;

type Result<T> = std::result::Result<T, CompilationError>;

/// Compile a member access expression.
///
/// Dispatches based on the kind of member access:
/// - Field: direct field or property getter
/// - Method: method call with arguments
pub fn compile_member<'ast>(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    member: &MemberExpr<'ast>,
) -> Result<ExprInfo> {
    // Compile the object expression first
    let obj_info = compiler.infer(member.object)?;

    match &member.member {
        MemberAccess::Field(ident) => {
            compile_field_access(compiler, &obj_info.data_type, ident.name, member.span)
        }
        MemberAccess::Method { name, args } => {
            calls::compile_method_call(compiler, &obj_info.data_type, name.name, args, member.span)
        }
    }
}

/// Compile field or property access.
///
/// Looks up the property by name and either:
/// - Emits GetField for direct fields
/// - Calls the getter method for virtual properties
fn compile_field_access(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    obj_type: &DataType,
    field_name: &str,
    span: Span,
) -> Result<ExprInfo> {
    // Get the class entry
    let class = compiler
        .ctx()
        .get_type(obj_type.type_hash)
        .and_then(|e| e.as_class())
        .ok_or_else(|| CompilationError::Other {
            message: format!("cannot access field '{}' on non-class type", field_name),
            span,
        })?;

    // Find the property by name
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

    // Check visibility (TODO: proper visibility checking with current context)

    if property.is_direct_field() {
        // Direct field access
        compiler.emitter().emit_get_field(field_index as u16);

        // Determine mutability based on object - use member source for ref return validation
        let result_type = property.data_type;
        let is_const = obj_type.is_effectively_const();
        Ok(ExprInfo::member(result_type, is_const))
    } else if let Some(getter_hash) = property.getter {
        // Virtual property with getter
        // Check const-correctness
        let getter =
            compiler
                .ctx()
                .get_function(getter_hash)
                .ok_or_else(|| CompilationError::Internal {
                    message: format!("Getter method not found: {:?}", getter_hash),
                })?;

        if obj_type.is_effectively_const() && !getter.def.is_const() {
            return Err(CompilationError::CannotModifyConst {
                message: format!(
                    "cannot call non-const getter for property '{}' on const object",
                    field_name
                ),
                span,
            });
        }

        // Call the getter
        compiler.emitter().emit_call_method(getter_hash, 0);

        Ok(ExprInfo::rvalue(property.data_type))
    } else {
        // Write-only property (has setter but no getter)
        Err(CompilationError::Other {
            message: format!("property '{}' is write-only", field_name),
            span,
        })
    }
}

/// Compile an index expression.
///
/// Tries operators in this order:
/// 1. opIndex (unified read/write, returns reference)
/// 2. get_opIndex (read-only, for read context)
///
/// Note: set_opIndex is used during assignment compilation (Task 43).
pub fn compile_index(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    index: &IndexExpr<'_>,
) -> Result<ExprInfo> {
    let span = index.span;

    // Compile the object expression
    let obj_info = compiler.infer(index.object)?;

    // Compile all index expressions - multi-dimensional indexing passes multiple args to opIndex
    let mut index_types = Vec::with_capacity(index.indices.len());
    for idx_item in index.indices {
        let info = compiler.infer(idx_item.index)?;
        index_types.push(info.data_type);
    }

    // Check for opIndex operators (extract what we need to avoid borrow conflicts)
    // Use behaviors.get_operator for operator lookup (not methods map)
    let (op_index_methods, get_opindex_methods, class_name) = {
        let class = compiler
            .ctx()
            .get_type(obj_info.data_type.type_hash)
            .and_then(|e| e.as_class())
            .ok_or_else(|| CompilationError::Other {
                message: "cannot index non-class type".to_string(),
                span,
            })?;

        // Look up operators from behaviors, falling back to methods for backwards compatibility
        let op_index = class
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
            .unwrap_or_default();

        let get_opindex = class
            .behaviors
            .get_operator(OperatorBehavior::OpIndexGet)
            .map(|v| v.to_vec())
            .or_else(|| {
                let methods = class.find_methods("get_opIndex");
                if methods.is_empty() {
                    None
                } else {
                    Some(methods.to_vec())
                }
            })
            .unwrap_or_default();

        (op_index, get_opindex, class.qualified_name.clone())
    };

    // Try opIndex first (unified read/write)
    if !op_index_methods.is_empty() {
        return compile_opindex(
            compiler,
            &obj_info.data_type,
            &op_index_methods,
            &index_types,
            span,
        );
    }

    // Try get_opIndex (read-only access)
    if !get_opindex_methods.is_empty() {
        return compile_opindex_get(
            compiler,
            &obj_info.data_type,
            &get_opindex_methods,
            &index_types,
            span,
        );
    }

    Err(CompilationError::Other {
        message: format!(
            "type '{}' does not support indexing (no opIndex or get_opIndex)",
            class_name
        ),
        span,
    })
}

/// Compile opIndex access (returns lvalue for read/write).
fn compile_opindex(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    obj_type: &DataType,
    candidates: &[TypeHash],
    index_types: &[DataType],
    span: Span,
) -> Result<ExprInfo> {
    use crate::overload::resolve_overload;

    let arg_count = index_types.len();

    // Filter candidates by const-ness to avoid ambiguous overloads
    // For mutable objects: prefer non-const methods, but allow const
    // For const objects: only const methods are valid
    let is_const_obj = obj_type.is_effectively_const();
    let filtered_candidates: Vec<TypeHash> = candidates
        .iter()
        .filter(|&&hash| {
            compiler
                .ctx()
                .get_function(hash)
                .is_some_and(|f| !is_const_obj || f.def.is_const())
        })
        .copied()
        .collect();

    // If mutable object and we have both const and non-const, prefer non-const
    let final_candidates = if !is_const_obj {
        let non_const: Vec<TypeHash> = filtered_candidates
            .iter()
            .filter(|&&hash| {
                compiler
                    .ctx()
                    .get_function(hash)
                    .is_some_and(|f| !f.def.is_const())
            })
            .copied()
            .collect();
        if !non_const.is_empty() {
            non_const
        } else {
            filtered_candidates
        }
    } else {
        filtered_candidates
    };

    if final_candidates.is_empty() {
        return Err(CompilationError::CannotModifyConst {
            message: "cannot call non-const opIndex on const object".to_string(),
            span,
        });
    }

    // Resolve overload with the filtered candidates
    let overload = resolve_overload(&final_candidates, index_types, compiler.ctx(), span)?;

    // Get return type
    let return_type = {
        let func = compiler
            .ctx()
            .get_function(overload.func_hash)
            .ok_or_else(|| CompilationError::Internal {
                message: format!("opIndex method not found: {:?}", overload.func_hash),
            })?;
        func.def.return_type
    };

    // Apply argument conversions
    for conv in overload.arg_conversions.iter().flatten() {
        super::emit_conversion(compiler.emitter(), conv);
    }

    // Emit method call
    compiler
        .emitter()
        .emit_call_method(overload.func_hash, arg_count as u8);

    // opIndex typically returns a reference to an element within the container,
    // so it's an lvalue with Member source (safe for reference returns).
    // The mutability depends on the object's const-ness.
    let is_const = obj_type.is_effectively_const();
    Ok(ExprInfo::member(return_type, is_const))
}

/// Compile get_opIndex access (read-only, returns rvalue).
fn compile_opindex_get(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    obj_type: &DataType,
    candidates: &[TypeHash],
    index_types: &[DataType],
    span: Span,
) -> Result<ExprInfo> {
    use crate::overload::resolve_overload;

    let arg_count = index_types.len();

    // Resolve overload with the index types as arguments
    let overload = resolve_overload(candidates, index_types, compiler.ctx(), span)?;

    // Const-correctness check and get return type
    let (is_const_method, return_type) = {
        let func = compiler
            .ctx()
            .get_function(overload.func_hash)
            .ok_or_else(|| CompilationError::Internal {
                message: format!("get_opIndex method not found: {:?}", overload.func_hash),
            })?;
        (func.def.is_const(), func.def.return_type)
    };

    if obj_type.is_effectively_const() && !is_const_method {
        return Err(CompilationError::CannotModifyConst {
            message: "cannot call non-const get_opIndex on const object".to_string(),
            span,
        });
    }

    // Apply argument conversions
    for conv in overload.arg_conversions.iter().flatten() {
        super::emit_conversion(compiler.emitter(), conv);
    }

    // Emit method call
    compiler
        .emitter()
        .emit_call_method(overload.func_hash, arg_count as u8);

    // get_opIndex returns an rvalue (copy)
    Ok(ExprInfo::rvalue(return_type))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::ConstantPool;
    use crate::context::CompilationContext;
    use crate::emit::BytecodeEmitter;
    use angelscript_core::{
        ClassEntry, DataType, FunctionDef, FunctionEntry, FunctionTraits, Param, PropertyEntry,
        TypeKind, Visibility, primitives,
    };
    use angelscript_registry::SymbolRegistry;

    fn create_test_context() -> (SymbolRegistry, ConstantPool) {
        (SymbolRegistry::with_primitives(), ConstantPool::new())
    }

    fn create_class_with_field(registry: &mut SymbolRegistry) -> TypeHash {
        let type_hash = TypeHash::from_name("Point");
        let mut class = ClassEntry::ffi("Point", TypeKind::script_object());

        // Add a direct field 'x'
        class.properties.push(PropertyEntry::field(
            "x",
            DataType::simple(primitives::INT32),
            Visibility::Public,
        ));

        registry.register_type(class.into()).unwrap();
        type_hash
    }

    fn create_class_with_property(registry: &mut SymbolRegistry) -> (TypeHash, TypeHash) {
        let type_hash = TypeHash::from_name("Container");
        let getter_hash = TypeHash::from_method(type_hash, "get_length", &[]);

        // Create class with virtual property
        let mut class = ClassEntry::ffi("Container", TypeKind::script_object());
        class.properties.push(PropertyEntry::read_only(
            "length",
            DataType::simple(primitives::INT32),
            getter_hash,
        ));
        class.add_method("get_length", getter_hash);
        registry.register_type(class.into()).unwrap();

        // Register getter method
        let mut traits = FunctionTraits::default();
        traits.is_const = true;
        let getter_def = FunctionDef::new(
            getter_hash,
            "get_length".to_string(),
            vec![],
            vec![],
            DataType::simple(primitives::INT32),
            Some(type_hash),
            traits,
            true,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(getter_def))
            .unwrap();

        (type_hash, getter_hash)
    }

    #[test]
    fn field_access_direct_field() {
        let (mut registry, mut constants) = create_test_context();
        let type_hash = create_class_with_field(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);
        let mut compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        let obj_type = DataType::simple(type_hash);
        let result = compile_field_access(&mut compiler, &obj_type, "x", Span::default());

        assert!(result.is_ok());
        let info = result.unwrap();
        assert!(info.is_lvalue);
        assert!(info.is_mutable);
        assert_eq!(info.data_type.type_hash, primitives::INT32);
    }

    #[test]
    fn field_access_const_object() {
        let (mut registry, mut constants) = create_test_context();
        let type_hash = create_class_with_field(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);
        let mut compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        let mut obj_type = DataType::simple(type_hash);
        obj_type.is_const = true;
        let result = compile_field_access(&mut compiler, &obj_type, "x", Span::default());

        assert!(result.is_ok());
        let info = result.unwrap();
        assert!(info.is_lvalue);
        assert!(!info.is_mutable); // Const object -> not mutable
    }

    #[test]
    fn field_access_unknown_field() {
        let (mut registry, mut constants) = create_test_context();
        let type_hash = create_class_with_field(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);
        let mut compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        let obj_type = DataType::simple(type_hash);
        let result = compile_field_access(&mut compiler, &obj_type, "unknown", Span::default());

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CompilationError::UnknownField { .. }
        ));
    }

    #[test]
    fn property_access_with_getter() {
        let (mut registry, mut constants) = create_test_context();
        let (type_hash, _) = create_class_with_property(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);
        let mut compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        let obj_type = DataType::simple(type_hash);
        let result = compile_field_access(&mut compiler, &obj_type, "length", Span::default());

        assert!(result.is_ok());
        let info = result.unwrap();
        assert!(!info.is_lvalue); // Getter returns rvalue
        assert_eq!(info.data_type.type_hash, primitives::INT32);
    }

    // =========================================================================
    // Index expression tests (opIndex/get_opIndex)
    // =========================================================================

    fn create_class_with_opindex(
        registry: &mut SymbolRegistry,
        is_const: bool,
    ) -> (TypeHash, TypeHash) {
        let type_hash = TypeHash::from_name("Array");
        let opindex_hash = TypeHash::from_method(type_hash, "opIndex", &[primitives::INT32]);

        let mut class = ClassEntry::ffi("Array", TypeKind::script_object());
        class.add_method("opIndex", opindex_hash);
        registry.register_type(class.into()).unwrap();

        // Register opIndex method
        let mut traits = FunctionTraits::default();
        traits.is_const = is_const;
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
            DataType::simple(primitives::INT32), // Returns int&
            Some(type_hash),
            traits,
            true,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(opindex_def))
            .unwrap();

        (type_hash, opindex_hash)
    }

    fn create_class_with_get_opindex(
        registry: &mut SymbolRegistry,
        is_const: bool,
    ) -> (TypeHash, TypeHash) {
        let type_hash = TypeHash::from_name("ReadOnlyArray");
        let get_opindex_hash =
            TypeHash::from_method(type_hash, "get_opIndex", &[primitives::INT32]);

        let mut class = ClassEntry::ffi("ReadOnlyArray", TypeKind::script_object());
        class.add_method("get_opIndex", get_opindex_hash);
        registry.register_type(class.into()).unwrap();

        // Register get_opIndex method
        let mut traits = FunctionTraits::default();
        traits.is_const = is_const;
        let get_opindex_def = FunctionDef::new(
            get_opindex_hash,
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
            traits,
            true,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(get_opindex_def))
            .unwrap();

        (type_hash, get_opindex_hash)
    }

    #[test]
    fn opindex_returns_lvalue() {
        let (mut registry, mut constants) = create_test_context();
        let (type_hash, _) = create_class_with_opindex(&mut registry, false);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);
        let mut compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        let obj_type = DataType::simple(type_hash);
        let index_types = vec![DataType::simple(primitives::INT32)];

        // Get the opIndex methods
        let class = compiler
            .ctx()
            .get_type(type_hash)
            .unwrap()
            .as_class()
            .unwrap();
        let candidates = class.find_methods("opIndex").to_vec();

        let result = compile_opindex(
            &mut compiler,
            &obj_type,
            &candidates,
            &index_types,
            Span::default(),
        );
        assert!(result.is_ok());
        let info = result.unwrap();
        assert!(info.is_lvalue);
        assert!(info.is_mutable);
    }

    #[test]
    fn opindex_const_object_returns_const_lvalue() {
        let (mut registry, mut constants) = create_test_context();
        let (type_hash, _) = create_class_with_opindex(&mut registry, true); // const method

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);
        let mut compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        let mut obj_type = DataType::simple(type_hash);
        obj_type.is_const = true;
        let index_types = vec![DataType::simple(primitives::INT32)];

        // Get the opIndex methods
        let class = compiler
            .ctx()
            .get_type(type_hash)
            .unwrap()
            .as_class()
            .unwrap();
        let candidates = class.find_methods("opIndex").to_vec();

        let result = compile_opindex(
            &mut compiler,
            &obj_type,
            &candidates,
            &index_types,
            Span::default(),
        );
        assert!(result.is_ok());
        let info = result.unwrap();
        assert!(info.is_lvalue);
        assert!(!info.is_mutable); // const object -> not mutable
    }

    #[test]
    fn opindex_non_const_on_const_object_rejected() {
        let (mut registry, mut constants) = create_test_context();
        let (type_hash, _) = create_class_with_opindex(&mut registry, false); // non-const method

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);
        let mut compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        let mut obj_type = DataType::simple(type_hash);
        obj_type.is_const = true;
        let index_types = vec![DataType::simple(primitives::INT32)];

        // Get the opIndex methods
        let class = compiler
            .ctx()
            .get_type(type_hash)
            .unwrap()
            .as_class()
            .unwrap();
        let candidates = class.find_methods("opIndex").to_vec();

        let result = compile_opindex(
            &mut compiler,
            &obj_type,
            &candidates,
            &index_types,
            Span::default(),
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CompilationError::CannotModifyConst { .. }
        ));
    }

    #[test]
    fn get_opindex_returns_rvalue() {
        let (mut registry, mut constants) = create_test_context();
        let (type_hash, _) = create_class_with_get_opindex(&mut registry, true);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);
        let mut compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        let obj_type = DataType::simple(type_hash);
        let index_types = vec![DataType::simple(primitives::INT32)];

        // Get the get_opIndex methods
        let class = compiler
            .ctx()
            .get_type(type_hash)
            .unwrap()
            .as_class()
            .unwrap();
        let candidates = class.find_methods("get_opIndex").to_vec();

        let result = compile_opindex_get(
            &mut compiler,
            &obj_type,
            &candidates,
            &index_types,
            Span::default(),
        );
        assert!(result.is_ok());
        let info = result.unwrap();
        assert!(!info.is_lvalue); // get_opIndex returns rvalue
    }

    #[test]
    fn get_opindex_non_const_on_const_object_rejected() {
        let (mut registry, mut constants) = create_test_context();
        let (type_hash, _) = create_class_with_get_opindex(&mut registry, false); // non-const method

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);
        let mut compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        let mut obj_type = DataType::simple(type_hash);
        obj_type.is_const = true;
        let index_types = vec![DataType::simple(primitives::INT32)];

        // Get the get_opIndex methods
        let class = compiler
            .ctx()
            .get_type(type_hash)
            .unwrap()
            .as_class()
            .unwrap();
        let candidates = class.find_methods("get_opIndex").to_vec();

        let result = compile_opindex_get(
            &mut compiler,
            &obj_type,
            &candidates,
            &index_types,
            Span::default(),
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CompilationError::CannotModifyConst { .. }
        ));
    }

    // =========================================================================
    // Virtual property const-correctness tests
    // =========================================================================

    fn create_class_with_non_const_getter(registry: &mut SymbolRegistry) -> (TypeHash, TypeHash) {
        let type_hash = TypeHash::from_name("MutableContainer");
        let getter_hash = TypeHash::from_method(type_hash, "get_value", &[]);

        let mut class = ClassEntry::ffi("MutableContainer", TypeKind::script_object());
        class.properties.push(PropertyEntry::read_only(
            "value",
            DataType::simple(primitives::INT32),
            getter_hash,
        ));
        class.add_method("get_value", getter_hash);
        registry.register_type(class.into()).unwrap();

        // Register NON-const getter method
        let mut traits = FunctionTraits::default();
        traits.is_const = false;
        let getter_def = FunctionDef::new(
            getter_hash,
            "get_value".to_string(),
            vec![],
            vec![],
            DataType::simple(primitives::INT32),
            Some(type_hash),
            traits,
            true,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(getter_def))
            .unwrap();

        (type_hash, getter_hash)
    }

    #[test]
    fn property_non_const_getter_on_const_object_rejected() {
        let (mut registry, mut constants) = create_test_context();
        let (type_hash, _) = create_class_with_non_const_getter(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);
        let mut compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        let mut obj_type = DataType::simple(type_hash);
        obj_type.is_const = true;
        let result = compile_field_access(&mut compiler, &obj_type, "value", Span::default());

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CompilationError::CannotModifyConst { .. }
        ));
    }

    #[test]
    fn property_non_const_getter_on_mutable_object_allowed() {
        let (mut registry, mut constants) = create_test_context();
        let (type_hash, _) = create_class_with_non_const_getter(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);
        let mut compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);

        let obj_type = DataType::simple(type_hash);
        let result = compile_field_access(&mut compiler, &obj_type, "value", Span::default());

        assert!(result.is_ok());
    }
}
