//! Variable declaration compilation.
//!
//! Handles variable declarations including:
//! - Explicit type declarations: `int x = 5;`
//! - Multiple declarators: `int x = 1, y = 2;`
//! - Auto type inference: `auto x = someFunction();`
//! - Default initialization: `int x;` (zero-initialized)
//! - Const variables: `const int x = 42;`

use angelscript_core::{CompilationError, DataType, Span};
use angelscript_parser::ast::{TypeBase, VarDeclStmt};

use crate::type_resolver::TypeResolver;

use super::{Result, StmtCompiler};

impl<'a, 'ctx, 'pool> StmtCompiler<'a, 'ctx, 'pool> {
    /// Compile a variable declaration statement.
    ///
    /// Supports multiple declarators, auto type inference, and default initialization.
    pub fn compile_var_decl<'ast>(&mut self, decl: &VarDeclStmt<'ast>) -> Result<()> {
        // Check if this is an auto type declaration
        let is_auto = matches!(decl.ty.base, TypeBase::Auto);

        // For non-auto types, resolve the base type once
        let base_type = if is_auto {
            None
        } else {
            Some(TypeResolver::new(self.ctx).resolve(&decl.ty)?)
        };

        // Process each declarator
        for var in decl.vars {
            let var_type = if is_auto {
                // Auto type - must have initializer
                let Some(init) = &var.init else {
                    return Err(CompilationError::Other {
                        message: "auto variable must have an initializer".to_string(),
                        span: var.span,
                    });
                };

                // Infer type from initializer
                let mut expr_compiler = self.expr_compiler();
                let info = expr_compiler.infer(init)?;
                info.data_type
            } else {
                base_type.unwrap()
            };

            // Check if the type is const from the declaration
            let is_const = decl.ty.is_const;

            // Declare the variable in the current scope
            let slot =
                self.ctx
                    .declare_local(var.name.name.to_string(), var_type, is_const, var.span)?;

            // Compile initializer or default initialization
            if let Some(init) = &var.init {
                // For auto type, the expression was already compiled during type inference
                // For explicit type, compile with type check
                if !is_auto {
                    let mut expr_compiler = self.expr_compiler();
                    expr_compiler.check(init, &var_type)?;
                }

                // For handles, AddRef before storing (we Release on scope exit)
                if var_type.is_handle {
                    let addref_hash = self.get_addref_behavior(var_type.type_hash, var.span)?;
                    self.emitter.emit_add_ref(addref_hash);
                }

                // Store the result in the local slot
                self.emitter.emit_set_local(slot);
            } else {
                // Default initialization
                self.emit_default_init(&var_type, slot, var.span)?;
            }

            // Mark the variable as initialized
            self.ctx.mark_local_initialized(var.name.name);
        }

        Ok(())
    }

    /// Emit default initialization for a variable.
    ///
    /// Initialization semantics:
    /// - Primitives: zero-initialized
    /// - Enums: first enum value
    /// - Handles: null
    /// - Value types: call default constructor (error if none exists)
    fn emit_default_init(&mut self, var_type: &DataType, slot: u32, span: Span) -> Result<()> {
        if var_type.is_primitive() {
            // Zero-initialize primitives
            self.emitter.emit_int(0);
            self.emitter.emit_set_local(slot);
            Ok(())
        } else if var_type.is_handle {
            // Null-initialize handles
            self.emitter.emit_null();
            self.emitter.emit_set_local(slot);
            Ok(())
        } else {
            // Look up the type to determine how to initialize
            let type_hash = var_type.type_hash;
            let type_entry =
                self.ctx
                    .get_type(type_hash)
                    .ok_or_else(|| CompilationError::Other {
                        message: "unknown type for default initialization".to_string(),
                        span,
                    })?;

            if let Some(enum_entry) = type_entry.as_enum() {
                // Enums initialize to first value (or 0 if empty)
                let value = enum_entry.values.first().map(|v| v.value).unwrap_or(0);
                self.emitter.emit_int(value);
                self.emitter.emit_set_local(slot);
                Ok(())
            } else if let Some(class) = type_entry.as_class() {
                // Value type - call default constructor
                if class.behaviors.constructors.is_empty() {
                    return Err(CompilationError::Other {
                        message: format!(
                            "type '{}' has no default constructor",
                            type_entry.qualified_name()
                        ),
                        span,
                    });
                }

                // Resolve default constructor (0 args) via overload resolution
                let overload = crate::overload::resolve_overload(
                    &class.behaviors.constructors,
                    &[],
                    self.ctx,
                    span,
                )?;

                self.emitter.emit_new(type_hash, overload.func_hash, 0);
                self.emitter.emit_set_local(slot);
                Ok(())
            } else {
                Err(CompilationError::Other {
                    message: format!(
                        "type '{}' cannot be default-initialized",
                        type_entry.qualified_name()
                    ),
                    span,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::ConstantPool;
    use crate::context::CompilationContext;
    use crate::emit::BytecodeEmitter;
    use angelscript_core::primitives;
    use angelscript_parser::ast::{
        Expr, Ident, LiteralExpr, LiteralKind, PrimitiveType, TypeExpr, VarDeclarator,
    };
    use angelscript_registry::SymbolRegistry;
    use bumpalo::Bump;

    fn create_test_context() -> (SymbolRegistry, ConstantPool) {
        (SymbolRegistry::with_primitives(), ConstantPool::new())
    }

    #[test]
    fn var_decl_with_int_initializer() {
        use crate::bytecode::OpCode;

        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // int x = 42;
        let init = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::default(),
        }));

        let vars = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("x", Span::default()),
            init: Some(init),
            span: Span::default(),
        }]);

        let decl = VarDeclStmt {
            ty: TypeExpr::primitive(PrimitiveType::Int, Span::default()),
            vars,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        compiler.compile_var_decl(&decl).unwrap();

        // Variable should be declared
        let var = ctx.get_local("x");
        assert!(var.is_some());
        assert_eq!(var.unwrap().data_type.type_hash, primitives::INT32);
        assert!(var.unwrap().is_initialized);

        // Bytecode: Constant(2) + SetLocal(2) = 4 bytes
        let chunk = emitter.finish();
        assert_eq!(chunk.len(), 4);
        assert_eq!(chunk.read_op(0), Some(OpCode::Constant));
        assert_eq!(chunk.read_byte(1), Some(0)); // Constant pool index
        assert_eq!(chunk.read_op(2), Some(OpCode::SetLocal));
        assert_eq!(chunk.read_byte(3), Some(0)); // Slot 0
    }

    #[test]
    fn var_decl_default_init_primitive() {
        use crate::bytecode::OpCode;

        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // int x;
        let vars = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("x", Span::default()),
            init: None,
            span: Span::default(),
        }]);

        let decl = VarDeclStmt {
            ty: TypeExpr::primitive(PrimitiveType::Int, Span::default()),
            vars,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        compiler.compile_var_decl(&decl).unwrap();

        let var = ctx.get_local("x");
        assert!(var.is_some());
        assert!(var.unwrap().is_initialized);

        // Bytecode: PushZero(1) + SetLocal(2) = 3 bytes (default init to 0)
        let chunk = emitter.finish();
        assert_eq!(chunk.len(), 3);
        assert_eq!(chunk.read_op(0), Some(OpCode::PushZero));
        assert_eq!(chunk.read_op(1), Some(OpCode::SetLocal));
        assert_eq!(chunk.read_byte(2), Some(0)); // Slot 0
    }

    #[test]
    fn var_decl_multiple_declarators() {
        use crate::bytecode::OpCode;

        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // int x = 1, y = 2;
        let init1 = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(1),
            span: Span::default(),
        }));
        let init2 = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(2),
            span: Span::default(),
        }));

        let vars = arena.alloc_slice_copy(&[
            VarDeclarator {
                name: Ident::new("x", Span::default()),
                init: Some(init1),
                span: Span::default(),
            },
            VarDeclarator {
                name: Ident::new("y", Span::default()),
                init: Some(init2),
                span: Span::default(),
            },
        ]);

        let decl = VarDeclStmt {
            ty: TypeExpr::primitive(PrimitiveType::Int, Span::default()),
            vars,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        compiler.compile_var_decl(&decl).unwrap();

        // Both variables should be declared
        assert!(ctx.get_local("x").is_some());
        assert!(ctx.get_local("y").is_some());

        // Check slots are different
        assert_eq!(ctx.get_local("x").unwrap().slot, 0);
        assert_eq!(ctx.get_local("y").unwrap().slot, 1);

        // Bytecode: x=1 uses PushOne(1) + SetLocal(2) = 3 bytes
        //           y=2 uses Constant(2) + SetLocal(2) = 4 bytes
        // Total: 7 bytes
        let chunk = emitter.finish();
        assert_eq!(chunk.len(), 7);
        assert_eq!(chunk.read_op(0), Some(OpCode::PushOne));
        assert_eq!(chunk.read_op(1), Some(OpCode::SetLocal));
        assert_eq!(chunk.read_byte(2), Some(0)); // Slot 0 for x
        assert_eq!(chunk.read_op(3), Some(OpCode::Constant));
        assert_eq!(chunk.read_byte(4), Some(0)); // Constant pool index for 2
        assert_eq!(chunk.read_op(5), Some(OpCode::SetLocal));
        assert_eq!(chunk.read_byte(6), Some(1)); // Slot 1 for y
    }

    #[test]
    fn var_decl_auto_type_inference() {
        use crate::bytecode::OpCode;

        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // auto x = 42;
        let init = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::default(),
        }));

        let vars = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("x", Span::default()),
            init: Some(init),
            span: Span::default(),
        }]);

        let decl = VarDeclStmt {
            ty: TypeExpr::new(false, None, TypeBase::Auto, &[], &[], Span::default()),
            vars,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        compiler.compile_var_decl(&decl).unwrap();

        // Variable should be inferred as int (small ints are INT32)
        let var = ctx.get_local("x");
        assert!(var.is_some());
        assert_eq!(var.unwrap().data_type.type_hash, primitives::INT32);

        // Bytecode: Constant(2) + SetLocal(2) = 4 bytes
        let chunk = emitter.finish();
        assert_eq!(chunk.len(), 4);
        assert_eq!(chunk.read_op(0), Some(OpCode::Constant));
        assert_eq!(chunk.read_op(2), Some(OpCode::SetLocal));
    }

    #[test]
    fn var_decl_auto_without_initializer_error() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // auto x; - should error
        let vars = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("x", Span::default()),
            init: None,
            span: Span::default(),
        }]);

        let decl = VarDeclStmt {
            ty: TypeExpr::new(false, None, TypeBase::Auto, &[], &[], Span::default()),
            vars,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        let result = compiler.compile_var_decl(&decl);
        assert!(result.is_err());
    }

    #[test]
    fn var_decl_const() {
        use crate::bytecode::OpCode;

        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // const int x = 42;
        let init = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::default(),
        }));

        let vars = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("x", Span::default()),
            init: Some(init),
            span: Span::default(),
        }]);

        let mut ty = TypeExpr::primitive(PrimitiveType::Int, Span::default());
        ty.is_const = true;

        let decl = VarDeclStmt {
            ty,
            vars,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        compiler.compile_var_decl(&decl).unwrap();

        let var = ctx.get_local("x");
        assert!(var.is_some());
        assert!(var.unwrap().is_const);

        // Bytecode: Constant(2) + SetLocal(2) = 4 bytes
        let chunk = emitter.finish();
        assert_eq!(chunk.len(), 4);
        assert_eq!(chunk.read_op(0), Some(OpCode::Constant));
        assert_eq!(chunk.read_op(2), Some(OpCode::SetLocal));
    }

    #[test]
    fn var_decl_redeclaration_error() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // First declaration: int x = 1;
        let init1 = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(1),
            span: Span::default(),
        }));

        let vars1 = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("x", Span::default()),
            init: Some(init1),
            span: Span::default(),
        }]);

        let decl1 = VarDeclStmt {
            ty: TypeExpr::primitive(PrimitiveType::Int, Span::default()),
            vars: vars1,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        compiler.compile_var_decl(&decl1).unwrap();

        // Second declaration at same scope: int x = 2; - should error
        let init2 = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(2),
            span: Span::default(),
        }));

        let vars2 = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("x", Span::default()),
            init: Some(init2),
            span: Span::default(),
        }]);

        let decl2 = VarDeclStmt {
            ty: TypeExpr::primitive(PrimitiveType::Int, Span::default()),
            vars: vars2,
            span: Span::default(),
        };

        let result = compiler.compile_var_decl(&decl2);
        assert!(matches!(
            result,
            Err(CompilationError::VariableRedeclaration { .. })
        ));
    }

    #[test]
    fn var_decl_float() {
        use crate::bytecode::OpCode;

        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // float x = 3.14;
        let init = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Float(3.14),
            span: Span::default(),
        }));

        let vars = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("x", Span::default()),
            init: Some(init),
            span: Span::default(),
        }]);

        let decl = VarDeclStmt {
            ty: TypeExpr::primitive(PrimitiveType::Float, Span::default()),
            vars,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        compiler.compile_var_decl(&decl).unwrap();

        let var = ctx.get_local("x");
        assert!(var.is_some());
        assert_eq!(var.unwrap().data_type.type_hash, primitives::FLOAT);

        // Bytecode: Constant(2) + SetLocal(2) = 4 bytes
        let chunk = emitter.finish();
        assert_eq!(chunk.len(), 4);
        assert_eq!(chunk.read_op(0), Some(OpCode::Constant));
        assert_eq!(chunk.read_op(2), Some(OpCode::SetLocal));
    }

    #[test]
    fn var_decl_bool() {
        use crate::bytecode::OpCode;

        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // bool x = true;
        let init = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(true),
            span: Span::default(),
        }));

        let vars = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("x", Span::default()),
            init: Some(init),
            span: Span::default(),
        }]);

        let decl = VarDeclStmt {
            ty: TypeExpr::primitive(PrimitiveType::Bool, Span::default()),
            vars,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        compiler.compile_var_decl(&decl).unwrap();

        let var = ctx.get_local("x");
        assert!(var.is_some());
        assert_eq!(var.unwrap().data_type.type_hash, primitives::BOOL);

        // Bytecode: PushTrue(1) + SetLocal(2) = 3 bytes
        let chunk = emitter.finish();
        assert_eq!(chunk.len(), 3);
        assert_eq!(chunk.read_op(0), Some(OpCode::PushTrue));
        assert_eq!(chunk.read_op(1), Some(OpCode::SetLocal));
        assert_eq!(chunk.read_byte(2), Some(0)); // Slot 0
    }

    #[test]
    fn var_decl_default_init_enum() {
        use crate::bytecode::OpCode;
        use angelscript_core::TypeHash;
        use angelscript_core::entries::EnumEntry;
        use angelscript_parser::ast::TypeBase;

        let arena = Bump::new();
        let mut registry = SymbolRegistry::with_primitives();

        // Register an enum with custom first value
        let color_hash = TypeHash::from_name("Color");
        let color_enum = EnumEntry::ffi("Color")
            .with_value("Red", 5)
            .with_value("Green", 10)
            .with_value("Blue", 15);
        registry.register_type(color_enum.into()).unwrap();

        let mut constants = ConstantPool::new();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // Color c; (should initialize to Red = 5)
        let vars = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("c", Span::default()),
            init: None,
            span: Span::default(),
        }]);

        let decl = VarDeclStmt {
            ty: TypeExpr::new(
                false,
                None,
                TypeBase::Named(Ident::new("Color", Span::default())),
                &[],
                &[],
                Span::default(),
            ),
            vars,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        compiler.compile_var_decl(&decl).unwrap();

        let var = ctx.get_local("c");
        assert!(var.is_some());
        assert_eq!(var.unwrap().data_type.type_hash, color_hash);

        // Bytecode: Constant(2, value=5) + SetLocal(2) = 4 bytes (5 is not 0 or 1)
        let chunk = emitter.finish();
        assert_eq!(chunk.len(), 4);
        assert_eq!(chunk.read_op(0), Some(OpCode::Constant));
        assert_eq!(chunk.read_op(2), Some(OpCode::SetLocal));
    }

    #[test]
    fn var_decl_default_init_value_type() {
        use crate::bytecode::OpCode;
        use angelscript_core::entries::{ClassEntry, FunctionEntry};
        use angelscript_core::{FunctionDef, FunctionTraits, TypeHash, TypeKind, Visibility};
        use angelscript_parser::ast::TypeBase;

        let arena = Bump::new();
        let mut registry = SymbolRegistry::with_primitives();

        // Register a value type with default constructor
        let vec2_hash = TypeHash::from_name("Vec2");
        let ctor_hash = TypeHash::from_constructor(vec2_hash, &[]);

        let mut vec2_class = ClassEntry::ffi("Vec2", TypeKind::value::<[f32; 2]>());
        vec2_class.behaviors.constructors.push(ctor_hash);
        registry.register_type(vec2_class.into()).unwrap();

        // Register the default constructor function
        let ctor_def = FunctionDef::new(
            ctor_hash,
            "Vec2".to_string(),
            vec![],
            vec![],
            DataType::simple(vec2_hash),
            None,
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        let ctor_entry = FunctionEntry::ffi(ctor_def);
        registry.register_function(ctor_entry).unwrap();

        let mut constants = ConstantPool::new();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // Vec2 v; (should call default constructor)
        let vars = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("v", Span::default()),
            init: None,
            span: Span::default(),
        }]);

        let decl = VarDeclStmt {
            ty: TypeExpr::new(
                false,
                None,
                TypeBase::Named(Ident::new("Vec2", Span::default())),
                &[],
                &[],
                Span::default(),
            ),
            vars,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        compiler.compile_var_decl(&decl).unwrap();

        let var = ctx.get_local("v");
        assert!(var.is_some());
        assert_eq!(var.unwrap().data_type.type_hash, vec2_hash);

        // Should emit New for the constructor
        let chunk = emitter.finish();
        assert!(chunk.len() > 0);
        // Look for New opcode
        let mut found_new = false;
        for i in 0..chunk.len() {
            if chunk.read_op(i) == Some(OpCode::New) {
                found_new = true;
                break;
            }
        }
        assert!(found_new, "Expected New opcode for constructor");
    }

    #[test]
    fn var_decl_default_init_no_constructor_error() {
        use angelscript_core::TypeKind;
        use angelscript_core::entries::ClassEntry;
        use angelscript_parser::ast::TypeBase;

        let arena = Bump::new();
        let mut registry = SymbolRegistry::with_primitives();

        // Register a value type WITHOUT default constructor
        let vec2_class = ClassEntry::ffi("Vec2", TypeKind::value::<[f32; 2]>());
        // Note: NOT adding any constructors
        registry.register_type(vec2_class.into()).unwrap();

        let mut constants = ConstantPool::new();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // Vec2 v; (should error - no default constructor)
        let vars = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("v", Span::default()),
            init: None,
            span: Span::default(),
        }]);

        let decl = VarDeclStmt {
            ty: TypeExpr::new(
                false,
                None,
                TypeBase::Named(Ident::new("Vec2", Span::default())),
                &[],
                &[],
                Span::default(),
            ),
            vars,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        let result = compiler.compile_var_decl(&decl);
        assert!(result.is_err());
    }

    #[test]
    fn var_decl_handle_emits_addref() {
        use crate::bytecode::OpCode;
        use angelscript_core::entries::ClassEntry;
        use angelscript_core::{TypeHash, TypeKind};
        use angelscript_parser::ast::{TypeBase, TypeSuffix};

        let arena = Bump::new();
        let mut registry = SymbolRegistry::with_primitives();

        // Register a reference type with addref behavior
        let addref_hash = TypeHash::from_name("Foo::AddRef");
        let mut foo_class = ClassEntry::ffi("Foo", TypeKind::reference());
        foo_class.behaviors.addref = Some(addref_hash);
        registry.register_type(foo_class.into()).unwrap();

        let mut constants = ConstantPool::new();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // Foo@ f = null; (handle with initializer should emit AddRef)
        let init = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Null,
            span: Span::default(),
        }));

        let vars = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("f", Span::default()),
            init: Some(init),
            span: Span::default(),
        }]);

        // Create handle type: Foo@ using suffix
        let suffixes = arena.alloc_slice_copy(&[TypeSuffix::Handle { is_const: false }]);
        let ty = TypeExpr::new(
            false,
            None,
            TypeBase::Named(Ident::new("Foo", Span::default())),
            &[],
            suffixes,
            Span::default(),
        );

        let decl = VarDeclStmt {
            ty,
            vars,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        assert!(compiler.compile_var_decl(&decl).is_ok());

        // Verify AddRef was emitted by checking bytecode
        let chunk = emitter.finish();
        let mut found_addref = false;
        for i in 0..chunk.len() {
            if chunk.read_op(i) == Some(OpCode::AddRef) {
                found_addref = true;
                break;
            }
        }
        assert!(
            found_addref,
            "Expected AddRef opcode for handle initialization"
        );
    }

    #[test]
    fn var_decl_handle_default_init_no_addref() {
        use crate::bytecode::OpCode;
        use angelscript_core::TypeKind;
        use angelscript_core::entries::ClassEntry;
        use angelscript_parser::ast::{TypeBase, TypeSuffix};

        let arena = Bump::new();
        let mut registry = SymbolRegistry::with_primitives();

        // Register a reference type
        let foo_class = ClassEntry::ffi("Foo", TypeKind::reference());
        registry.register_type(foo_class.into()).unwrap();

        let mut constants = ConstantPool::new();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // Foo@ f; (default init to null - no AddRef needed for null)
        let vars = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("f", Span::default()),
            init: None,
            span: Span::default(),
        }]);

        // Create handle type: Foo@ using suffix
        let suffixes = arena.alloc_slice_copy(&[TypeSuffix::Handle { is_const: false }]);
        let ty = TypeExpr::new(
            false,
            None,
            TypeBase::Named(Ident::new("Foo", Span::default())),
            &[],
            suffixes,
            Span::default(),
        );

        let decl = VarDeclStmt {
            ty,
            vars,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        assert!(compiler.compile_var_decl(&decl).is_ok());

        // Verify NO AddRef was emitted (null doesn't need AddRef)
        let chunk = emitter.finish();
        let mut found_addref = false;
        for i in 0..chunk.len() {
            if chunk.read_op(i) == Some(OpCode::AddRef) {
                found_addref = true;
                break;
            }
        }
        assert!(
            !found_addref,
            "Expected no AddRef opcode for null default initialization"
        );
    }
}
