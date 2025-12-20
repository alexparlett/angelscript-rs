//! Field initialization for class constructors.
//!
//! AngelScript initializes class members in a specific order:
//! 1. Fields without explicit initialization (in declaration order)
//! 2. Base class initialization (via implicit or explicit `super()`)
//! 3. Fields with explicit initialization (in declaration order)
//!
//! This module handles compiling field initializers into constructor bytecode.

use angelscript_core::{CompilationError, DataType, Span, TypeHash};
use angelscript_parser::ast::{ClassDecl, ClassMember, Expr, Stmt};

use crate::context::CompilationContext;
use crate::emit::BytecodeEmitter;
use crate::expr::ExprCompiler;
use crate::type_resolver::TypeResolver;

type Result<T> = std::result::Result<T, CompilationError>;

/// Information about a field that needs initialization.
#[derive(Debug)]
pub struct FieldInit<'ast> {
    /// Field index in the class properties array.
    pub field_index: u16,
    /// Field type.
    pub data_type: DataType,
    /// Initializer expression (if any).
    pub init: Option<&'ast Expr<'ast>>,
    /// Source span for error reporting.
    pub span: Span,
}

/// Collect field initializers from a class declaration.
///
/// Returns two lists: fields without explicit init (go first) and
/// fields with explicit init (go after base class).
pub fn collect_field_inits<'ast>(
    ctx: &mut CompilationContext<'_>,
    class: &ClassDecl<'ast>,
) -> Result<(Vec<FieldInit<'ast>>, Vec<FieldInit<'ast>>)> {
    let mut without_init = Vec::new();
    let mut with_init = Vec::new();

    let mut resolver = TypeResolver::new(ctx);
    let mut field_index = 0u16;

    for member in class.members {
        if let ClassMember::Field(field) = member {
            let data_type = resolver.resolve(&field.ty)?;

            let field_info = FieldInit {
                field_index,
                data_type,
                init: field.init,
                span: field.span,
            };

            if field.init.is_some() {
                with_init.push(field_info);
            } else {
                without_init.push(field_info);
            }

            field_index += 1;
        }
    }

    Ok((without_init, with_init))
}

/// Compile field initializers that run before base class initialization.
///
/// These are fields WITHOUT explicit initialization - they get default values.
/// For now, we skip these as they're default-initialized by the VM.
pub fn compile_pre_base_inits(
    _ctx: &mut CompilationContext<'_>,
    _emitter: &mut BytecodeEmitter<'_>,
    _class_hash: TypeHash,
    _fields: &[FieldInit<'_>],
) -> Result<()> {
    // Fields without explicit init are default-initialized by the VM.
    // No bytecode needed for primitive types (they're zero-initialized).
    // For reference types, handle fields default to null.
    Ok(())
}

/// Compile field initializers that run after base class initialization.
///
/// These are fields WITH explicit initialization expressions.
pub fn compile_post_base_inits<'ast>(
    ctx: &mut CompilationContext<'_>,
    emitter: &mut BytecodeEmitter<'_>,
    class_hash: TypeHash,
    fields: &[FieldInit<'ast>],
) -> Result<()> {
    for field in fields {
        if let Some(init_expr) = field.init {
            // Load 'this' reference
            emitter.emit_get_this();

            // Compile the initializer expression
            {
                let mut expr_compiler = ExprCompiler::new(ctx, emitter, Some(class_hash));
                expr_compiler.check(init_expr, &field.data_type)?;
            }

            // Store to field
            emitter.emit_set_field(field.field_index);
        }
    }

    Ok(())
}

/// Check if a constructor body has an explicit super() call.
///
/// Returns the index of the super() call statement if found.
pub fn find_super_call<'ast>(body: &angelscript_parser::ast::Block<'ast>) -> Option<usize> {
    for (idx, stmt) in body.stmts.iter().enumerate() {
        if let Stmt::Expr(expr_stmt) = stmt {
            if let Some(expr) = expr_stmt.expr {
                if is_super_call(expr) {
                    return Some(idx);
                }
            }
        }
    }

    None
}

/// Check if an expression is a super() call.
fn is_super_call(expr: &Expr<'_>) -> bool {
    if let Expr::Call(call) = expr {
        if let Expr::Ident(ident) = call.callee {
            return ident.ident.name == "super";
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_parser::Parser;
    use angelscript_parser::ast::Item;
    use angelscript_registry::SymbolRegistry;
    use bumpalo::Bump;

    // =========================================================================
    // Basic Declaration Order Tests
    // Doc: "Members without explicit initialization follow declaration order,
    //       while explicitly initialized members come last."
    // =========================================================================

    #[test]
    fn collect_fields_partitions_by_init() {
        // Test: class Foo { int a; int b = 10; int c; int d = 20; }
        // Expected order: a, c (no init) then b, d (with init)
        let arena = Bump::new();
        let source = r#"
            class Foo {
                int a;
                int b = 10;
                int c;
                int d = 20;
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();
        let class = match &script.items()[0] {
            Item::Class(c) => c,
            _ => panic!("expected class"),
        };

        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);

        let (without, with) = collect_field_inits(&mut ctx, class).unwrap();

        // a and c have no init - these go first
        assert_eq!(without.len(), 2);
        assert_eq!(without[0].field_index, 0); // a
        assert_eq!(without[1].field_index, 2); // c

        // b and d have init - these go after base class
        assert_eq!(with.len(), 2);
        assert_eq!(with[0].field_index, 1); // b
        assert_eq!(with[1].field_index, 3); // d
    }

    #[test]
    fn collect_fields_preserves_declaration_order_within_groups() {
        // Doc example: "The order of this class will be: a, c, b, d"
        // where a, c have no init and b = a, d = b have init
        let arena = Bump::new();
        let source = r#"
            class Foo {
                int a;
                int b = 1;
                int c;
                int d = 2;
                int e;
                int f = 3;
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();
        let class = match &script.items()[0] {
            Item::Class(c) => c,
            _ => panic!("expected class"),
        };

        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);

        let (without, with) = collect_field_inits(&mut ctx, class).unwrap();

        // Without init: a(0), c(2), e(4) - in declaration order
        assert_eq!(without.len(), 3);
        assert_eq!(without[0].field_index, 0); // a
        assert_eq!(without[1].field_index, 2); // c
        assert_eq!(without[2].field_index, 4); // e

        // With init: b(1), d(3), f(5) - in declaration order
        assert_eq!(with.len(), 3);
        assert_eq!(with[0].field_index, 1); // b
        assert_eq!(with[1].field_index, 3); // d
        assert_eq!(with[2].field_index, 5); // f
    }

    #[test]
    fn collect_fields_all_without_init() {
        // Edge case: all fields have no initializer
        let arena = Bump::new();
        let source = r#"
            class Foo {
                int a;
                int b;
                int c;
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();
        let class = match &script.items()[0] {
            Item::Class(c) => c,
            _ => panic!("expected class"),
        };

        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);

        let (without, with) = collect_field_inits(&mut ctx, class).unwrap();

        assert_eq!(without.len(), 3);
        assert_eq!(with.len(), 0);
    }

    #[test]
    fn collect_fields_all_with_init() {
        // Edge case: all fields have initializers
        let arena = Bump::new();
        let source = r#"
            class Foo {
                int a = 1;
                int b = 2;
                int c = 3;
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();
        let class = match &script.items()[0] {
            Item::Class(c) => c,
            _ => panic!("expected class"),
        };

        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);

        let (without, with) = collect_field_inits(&mut ctx, class).unwrap();

        assert_eq!(without.len(), 0);
        assert_eq!(with.len(), 3);
        // Verify declaration order is preserved
        assert_eq!(with[0].field_index, 0); // a
        assert_eq!(with[1].field_index, 1); // b
        assert_eq!(with[2].field_index, 2); // c
    }

    #[test]
    fn collect_fields_empty_class() {
        // Edge case: class with no fields
        let arena = Bump::new();
        let source = r#"
            class Foo {
                void doSomething() {}
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();
        let class = match &script.items()[0] {
            Item::Class(c) => c,
            _ => panic!("expected class"),
        };

        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);

        let (without, with) = collect_field_inits(&mut ctx, class).unwrap();

        assert_eq!(without.len(), 0);
        assert_eq!(with.len(), 0);
    }

    // =========================================================================
    // Super() Call Detection Tests
    // Doc: "Base class initialization via super() must occur before accessing
    //       base class members in derived class."
    // =========================================================================

    #[test]
    fn find_super_call_at_start() {
        let arena = Bump::new();
        let source = r#"
            class Foo : Bar {
                Foo() {
                    super();
                    int x = 5;
                }
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();
        let class = match &script.items()[0] {
            Item::Class(c) => c,
            _ => panic!("expected class"),
        };

        for member in class.members {
            if let ClassMember::Method(func) = member {
                if func.is_constructor() {
                    if let Some(body) = &func.body {
                        let idx = find_super_call(body);
                        assert_eq!(idx, Some(0)); // super() is first
                        return;
                    }
                }
            }
        }
        panic!("No constructor found");
    }

    #[test]
    fn find_super_call_in_middle() {
        let arena = Bump::new();
        let source = r#"
            class Foo : Bar {
                Foo() {
                    int x = 5;
                    super();
                    int y = 10;
                }
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();
        let class = match &script.items()[0] {
            Item::Class(c) => c,
            _ => panic!("expected class"),
        };

        for member in class.members {
            if let ClassMember::Method(func) = member {
                if func.is_constructor() {
                    if let Some(body) = &func.body {
                        let idx = find_super_call(body);
                        assert_eq!(idx, Some(1)); // super() is at index 1
                        return;
                    }
                }
            }
        }
        panic!("No constructor found");
    }

    #[test]
    fn find_super_call_at_end() {
        let arena = Bump::new();
        let source = r#"
            class Foo : Bar {
                Foo() {
                    int x = 5;
                    int y = 10;
                    super();
                }
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();
        let class = match &script.items()[0] {
            Item::Class(c) => c,
            _ => panic!("expected class"),
        };

        for member in class.members {
            if let ClassMember::Method(func) = member {
                if func.is_constructor() {
                    if let Some(body) = &func.body {
                        let idx = find_super_call(body);
                        assert_eq!(idx, Some(2)); // super() is last
                        return;
                    }
                }
            }
        }
        panic!("No constructor found");
    }

    #[test]
    fn find_super_call_with_arguments() {
        // super() can take arguments for non-default base constructors
        let arena = Bump::new();
        let source = r#"
            class Foo : Bar {
                Foo(int x) {
                    super(x, 10);
                }
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();
        let class = match &script.items()[0] {
            Item::Class(c) => c,
            _ => panic!("expected class"),
        };

        for member in class.members {
            if let ClassMember::Method(func) = member {
                if func.is_constructor() {
                    if let Some(body) = &func.body {
                        let idx = find_super_call(body);
                        assert_eq!(idx, Some(0));
                        return;
                    }
                }
            }
        }
        panic!("No constructor found");
    }

    #[test]
    fn find_super_call_absent() {
        // No base class, no super() call
        let arena = Bump::new();
        let source = r#"
            class Foo {
                Foo() {
                    int x = 5;
                }
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();
        let class = match &script.items()[0] {
            Item::Class(c) => c,
            _ => panic!("expected class"),
        };

        for member in class.members {
            if let ClassMember::Method(func) = member {
                if func.is_constructor() {
                    if let Some(body) = &func.body {
                        let idx = find_super_call(body);
                        assert!(idx.is_none());
                        return;
                    }
                }
            }
        }
        panic!("No constructor found");
    }

    #[test]
    fn find_super_call_empty_constructor() {
        let arena = Bump::new();
        let source = r#"
            class Foo {
                Foo() {}
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();
        let class = match &script.items()[0] {
            Item::Class(c) => c,
            _ => panic!("expected class"),
        };

        for member in class.members {
            if let ClassMember::Method(func) = member {
                if func.is_constructor() {
                    if let Some(body) = &func.body {
                        let idx = find_super_call(body);
                        assert!(idx.is_none());
                        return;
                    }
                }
            }
        }
        panic!("No constructor found");
    }

    // =========================================================================
    // Complex Expression Initializer Tests
    // Doc: "Members explicitly initialized in constructor body remain
    //       uninitialized until that statement executes."
    // =========================================================================

    #[test]
    fn collect_fields_with_complex_expressions() {
        // Field initializers can be complex expressions
        let arena = Bump::new();
        let source = r#"
            class Foo {
                int a = 1 + 2 * 3;
                int b;
                int c = 10;
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();
        let class = match &script.items()[0] {
            Item::Class(c) => c,
            _ => panic!("expected class"),
        };

        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);

        let (without, with) = collect_field_inits(&mut ctx, class).unwrap();

        // b has no init
        assert_eq!(without.len(), 1);
        assert_eq!(without[0].field_index, 1); // b

        // a and c have init (regardless of complexity)
        assert_eq!(with.len(), 2);
        assert_eq!(with[0].field_index, 0); // a
        assert_eq!(with[1].field_index, 2); // c

        // Verify init expressions are captured
        assert!(with[0].init.is_some());
        assert!(with[1].init.is_some());
    }

    // =========================================================================
    // Multiple Constructor Tests
    // Each constructor should use the same field initialization logic
    // =========================================================================

    #[test]
    fn find_super_call_multiple_constructors() {
        let arena = Bump::new();
        let source = r#"
            class Foo : Bar {
                Foo() {
                    super();
                }
                Foo(int x) {
                    int y = x;
                    super(x);
                }
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();
        let class = match &script.items()[0] {
            Item::Class(c) => c,
            _ => panic!("expected class"),
        };

        let mut super_positions = Vec::new();
        for member in class.members {
            if let ClassMember::Method(func) = member {
                if func.is_constructor() {
                    if let Some(body) = &func.body {
                        super_positions.push(find_super_call(body));
                    }
                }
            }
        }

        // First constructor: super() at index 0
        // Second constructor: super(x) at index 1
        assert_eq!(super_positions.len(), 2);
        assert_eq!(super_positions[0], Some(0));
        assert_eq!(super_positions[1], Some(1));
    }
}
