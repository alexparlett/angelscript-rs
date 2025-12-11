# Task 41c: Validate Script Class Inheritance Rules

## Overview

Add validation to the registration pass to enforce AngelScript's inheritance rules: script classes can only extend other script classes OR implement interfaces. They cannot extend FFI classes.

## Context

Currently, `RegistrationPass::resolve_inheritance()` accepts any class as a base class without checking whether it's an FFI class or script class. This allows invalid inheritance like:

```angelscript
// FFI class registered from Rust
// class GameObject { ... }

// INVALID: Script class extending FFI class
class Player : GameObject {  // Should be rejected!
    // ...
}
```

**Valid patterns:**
- Script class extends script class: ✅ `class Player : Entity { }`
- Script class implements interface: ✅ `class Player : IDrawable { }`
- FFI class as base: ❌ Not allowed for script classes

## Current Code

In [registration.rs:256-295](crates/angelscript-compiler/src/passes/registration.rs#L256-L295):

```rust
fn resolve_inheritance(&mut self, class: &ClassDecl<'_>) -> (Option<TypeHash>, Vec<TypeHash>) {
    let mut base_class = None;
    let mut interfaces = Vec::new();

    for (i, inherit_expr) in class.inheritance.iter().enumerate() {
        let type_name = self.ident_expr_to_string(inherit_expr);

        if let Some(hash) = self.ctx.resolve_type(&type_name) {
            if let Some(entry) = self.ctx.get_type(hash) {
                if entry.is_interface() {
                    interfaces.push(hash);
                } else if entry.is_class() {
                    // BUG: Doesn't check if it's an FFI class!
                    if i == 0 && base_class.is_none() {
                        base_class = Some(hash);
                    } else {
                        // Multiple inheritance error
                    }
                }
            }
        }
    }

    (base_class, interfaces)
}
```

## Solution

Add validation when a base class is found to check if it's a script class:

```rust
} else if entry.is_class() {
    if i == 0 && base_class.is_none() {
        // NEW: Validate that script classes can only extend script classes
        if let Some(class_entry) = entry.as_class() {
            if class_entry.source.is_ffi() {
                self.ctx.add_error(CompilationError::InvalidOperation {
                    message: format!(
                        "script class '{}' cannot extend FFI class '{}'; script classes can only extend other script classes or implement interfaces",
                        class.name.name,
                        type_name
                    ),
                    span: class.span,
                });
                // Don't set base_class, skip this invalid inheritance
                continue;
            }
        }
        base_class = Some(hash);
    } else {
        // Multiple inheritance error (existing code)
    }
}
```

## Files to Modify

- [crates/angelscript-compiler/src/passes/registration.rs](crates/angelscript-compiler/src/passes/registration.rs#L256-L295)
  - Update `resolve_inheritance()` method
  - Add validation check for FFI base classes

## Testing

Add test to `registration.rs` tests:

```rust
#[test]
fn register_class_cannot_extend_ffi_class() {
    let mut registry = SymbolRegistry::with_primitives();

    // Register an FFI class
    let ffi_class = ClassEntry::ffi("GameObject", TypeKind::reference());
    registry.register_type(ffi_class.into()).unwrap();

    // Try to extend it from script
    let source = r#"
        class Player : GameObject {
            void update() {}
        }
    "#;
    let arena = bumpalo::Bump::new();
    let script = Parser::parse(source, &arena).unwrap();

    let mut ctx = CompilationContext::new(&registry);
    let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
    let output = pass.run(&script);

    // Should have an error
    assert_eq!(output.errors.len(), 1);
    match &output.errors[0] {
        CompilationError::InvalidOperation { message, .. } => {
            assert!(message.contains("cannot extend FFI class"));
        }
        other => panic!("Expected InvalidOperation error, got: {:?}", other),
    }
}

#[test]
fn register_class_can_extend_script_class() {
    let source = r#"
        class Entity {
            void update() {}
        }

        class Player : Entity {
            void render() {}
        }
    "#;
    let arena = bumpalo::Bump::new();
    let script = Parser::parse(source, &arena).unwrap();

    let registry = SymbolRegistry::with_primitives();
    let mut ctx = CompilationContext::new(&registry);
    let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
    let output = pass.run(&script);

    // Should succeed with no errors
    assert_eq!(output.errors.len(), 0);
    assert_eq!(output.types_registered, 2);
}

#[test]
fn register_class_can_implement_interface() {
    let mut registry = SymbolRegistry::with_primitives();

    // Register an FFI interface
    let interface = InterfaceEntry::ffi("IDrawable")
        .with_method("draw", TypeHash::from_function("IDrawable::draw", &[]));
    registry.register_type(interface.into()).unwrap();

    let source = r#"
        class Sprite : IDrawable {
            void draw() {}
        }
    "#;
    let arena = bumpalo::Bump::new();
    let script = Parser::parse(source, &arena).unwrap();

    let mut ctx = CompilationContext::new(&registry);
    let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
    let output = pass.run(&script);

    // Should succeed - interfaces are OK
    assert_eq!(output.errors.len(), 0);
}
```

## Acceptance Criteria

- [ ] Validation added to `resolve_inheritance()`
- [ ] Script class extending FFI class produces clear error message
- [ ] Script class extending script class still works
- [ ] Script class implementing FFI interface still works
- [ ] Tests added for all three scenarios
- [ ] All existing tests still pass
- [ ] No clippy warnings

## Notes

- This is a validation issue, not a type completion issue
- Task 41b (Type Completion Pass) correctly assumes valid inheritance
- This fix belongs in the registration pass where inheritance is first established
- Error message should be clear about what's allowed vs. not allowed

## Priority

Medium - This is a correctness issue but doesn't affect most common use cases (script-to-script inheritance). Should be fixed before production use.
