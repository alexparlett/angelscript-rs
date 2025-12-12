# Task 41c: Validate Script Class Inheritance Rules

## Overview

Add validation to the registration pass to enforce AngelScript's inheritance and mixin rules:
1. Script classes can only extend other script classes OR implement interfaces (not FFI classes)
2. Final classes cannot be inherited from
3. Mixin classes are not real types and cannot be instantiated
4. Mixin classes cannot inherit from other classes

## Context

Currently, `RegistrationPass::resolve_inheritance()` accepts any class as a base class without checking:
1. Whether it's an FFI class (script classes cannot extend FFI classes)
2. Whether it's marked `final` (final classes cannot be inherited from)

This allows invalid inheritance like:

```angelscript
// FFI class registered from Rust
// class GameObject { ... }

// INVALID: Script class extending FFI class
class Player : GameObject {  // Should be rejected!
    // ...
}

// Script class marked final
final class Entity { }

// INVALID: Extending a final class
class Player : Entity {  // Should be rejected!
    // ...
}
```

**Valid patterns:**
- Script class extends script class: ✅ `class Player : Entity { }`
- Script class implements interface: ✅ `class Player : IDrawable { }`
- Script class includes mixin: ✅ `class Player : PlayerMixin { }`
- FFI class as base: ❌ Not allowed for script classes
- Final class as base: ❌ Not allowed (script or FFI)
- Mixin inherits from class: ❌ Not allowed
- Mixin instantiation: ❌ `PlayerMixin m;` is not allowed

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

Add validation when a base class is found to check:
1. If it's an FFI class (script classes can only extend script classes)
2. If it's marked `final` (final classes cannot be inherited from)

```rust
} else if entry.is_class() {
    if i == 0 && base_class.is_none() {
        // NEW: Validate inheritance rules
        if let Some(class_entry) = entry.as_class() {
            // Rule 1: Script classes cannot extend FFI classes
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

            // Rule 2: Final classes cannot be inherited from
            if class_entry.is_final {
                self.ctx.add_error(CompilationError::InvalidOperation {
                    message: format!(
                        "class '{}' cannot extend final class '{}'",
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

### Mixin Class Validation

Mixin classes have special rules that need validation:

**Rule 3: Mixin classes cannot inherit from other classes**

When registering a mixin class, check that it has no base class:

```rust
fn register_class(&mut self, class: &ClassDecl<'_>) {
    // ... existing code ...

    let (base_class, interfaces) = self.resolve_inheritance(class);

    // NEW: Validate mixin class rules
    if class.is_mixin {
        // Rule 3: Mixins cannot inherit from classes
        if base_class.is_some() {
            self.ctx.add_error(CompilationError::InvalidOperation {
                message: format!(
                    "mixin class '{}' cannot inherit from other classes; mixins can only declare interfaces",
                    class.name.name
                ),
                span: class.span,
            });
            // Don't set base_class for the mixin
            base_class = None;
        }
    }

    // ... rest of registration ...
}
```

**Rule 4: Mixin classes cannot be instantiated**

This validation happens during compilation (not registration), when encountering variable declarations or object construction:

```rust
// In compilation pass, when checking type for instantiation
fn check_instantiable(&self, type_hash: TypeHash, span: Span) -> Result<()> {
    if let Some(entry) = self.ctx.get_type(type_hash) {
        if let Some(class_entry) = entry.as_class() {
            if class_entry.is_mixin {
                return Err(CompilationError::InvalidOperation {
                    message: format!(
                        "cannot instantiate mixin class '{}'; mixins are not real types",
                        class_entry.name
                    ),
                    span,
                });
            }
        }
    }
    Ok(())
}
```

**Mixin Inclusion Logic (Type Completion Pass)**

When a class includes a mixin, the type completion pass needs special handling:

1. Copy all methods from mixin to including class
2. Copy all properties from mixin to including class (unless already declared)
3. Mixin methods **override** inherited methods from base classes
4. Mixin properties are **not** included if already inherited from base class
5. Add mixin's required interfaces to the including class

This is different from regular inheritance and needs to be handled in the type completion pass.

## Files to Modify

- [crates/angelscript-compiler/src/passes/registration.rs](crates/angelscript-compiler/src/passes/registration.rs#L256-L295)
  - Update `resolve_inheritance()` method
  - Add validation check for FFI base classes

## Testing

Add tests to `registration.rs` tests:

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
fn register_class_cannot_extend_final_class() {
    let source = r#"
        final class Entity {
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

    // Should have an error
    assert_eq!(output.errors.len(), 1);
    match &output.errors[0] {
        CompilationError::InvalidOperation { message, .. } => {
            assert!(message.contains("cannot extend final class"));
        }
        other => panic!("Expected InvalidOperation error, got: {:?}", other),
    }
}

#[test]
fn register_class_cannot_extend_final_ffi_class() {
    let mut registry = SymbolRegistry::with_primitives();

    // Register a final FFI class
    let ffi_class = ClassEntry::ffi("GameObject", TypeKind::reference())
        .as_final();
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

    // Should have TWO errors: FFI class + final
    // (or just one if we check final first)
    assert!(output.errors.len() >= 1);
    assert!(output.errors.iter().any(|e| {
        matches!(e, CompilationError::InvalidOperation { message, .. }
            if message.contains("final") || message.contains("FFI"))
    }));
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

#[test]
fn register_mixin_cannot_inherit_from_class() {
    let source = r#"
        class Entity {
            void update() {}
        }

        mixin class PlayerMixin : Entity {
            void render() {}
        }
    "#;
    let arena = bumpalo::Bump::new();
    let script = Parser::parse(source, &arena).unwrap();

    let registry = SymbolRegistry::with_primitives();
    let mut ctx = CompilationContext::new(&registry);
    let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
    let output = pass.run(&script);

    // Should have an error
    assert_eq!(output.errors.len(), 1);
    match &output.errors[0] {
        CompilationError::InvalidOperation { message, .. } => {
            assert!(message.contains("mixin") && message.contains("cannot inherit"));
        }
        other => panic!("Expected InvalidOperation error, got: {:?}", other),
    }
}

#[test]
fn register_mixin_can_declare_interfaces() {
    let mut registry = SymbolRegistry::with_primitives();

    // Register an FFI interface
    let interface = InterfaceEntry::ffi("IDrawable")
        .with_method("draw", TypeHash::from_function("IDrawable::draw", &[]));
    registry.register_type(interface.into()).unwrap();

    let source = r#"
        mixin class RenderMixin : IDrawable {
            void draw() { /* default implementation */ }
        }

        class Player : RenderMixin {
            // Gets draw() from mixin
        }
    "#;
    let arena = bumpalo::Bump::new();
    let script = Parser::parse(source, &arena).unwrap();

    let mut ctx = CompilationContext::new(&registry);
    let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
    let output = pass.run(&script);

    // Should succeed - mixins can declare interfaces
    assert_eq!(output.errors.len(), 0);
}

#[test]
fn compile_cannot_instantiate_mixin() {
    // NOTE: This test belongs in compilation pass tests, not registration
    let source = r#"
        mixin class PlayerMixin {
            void render() {}
        }

        void test() {
            PlayerMixin m;  // ERROR: cannot instantiate mixin
        }
    "#;
    let arena = bumpalo::Bump::new();
    let script = Parser::parse(source, &arena).unwrap();

    let registry = SymbolRegistry::with_primitives();
    let mut ctx = CompilationContext::new(&registry);

    // Registration should succeed
    let reg_pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
    let reg_output = reg_pass.run(&script);
    assert_eq!(reg_output.errors.len(), 0);

    // Compilation should fail when trying to instantiate
    let comp_pass = CompilationPass::new(&mut ctx, UnitId::new(0));
    let comp_output = comp_pass.run(&script);

    assert!(comp_output.errors.len() > 0);
    assert!(comp_output.errors.iter().any(|e| {
        matches!(e, CompilationError::InvalidOperation { message, .. }
            if message.contains("cannot instantiate mixin"))
    }));
}
```

## Acceptance Criteria

### Validation Rules (Phase 1 - Registration)
- [ ] Script class extending FFI class produces clear error message
- [ ] Script class extending final class produces clear error message
- [ ] Script class extending script class still works
- [ ] Script class implementing FFI interface still works
- [ ] Mixin class inheriting from regular class produces clear error message
- [ ] Mixin class can declare interfaces (allowed)
- [ ] Class can include mixin class (allowed)

### Validation Rules (Phase 2 - Compilation)
- [ ] Instantiating a mixin class produces clear error message

### Additional Validations (Future Enhancement)
- [ ] Final methods cannot be overridden
- [ ] `override` keyword is only used when actually overriding an inherited method
- [ ] Abstract classes without all interface methods implemented must be marked abstract
- [ ] Non-abstract classes must implement all inherited abstract/interface methods

### Testing & Quality
- [ ] Tests added for all validation scenarios
- [ ] All existing tests still pass
- [ ] No clippy warnings

## Notes

- This is a validation issue, not a type completion issue
- Task 41b (Type Completion Pass) correctly assumes valid inheritance
- This fix belongs in the registration pass where inheritance is first established
- Error messages should be clear about what's allowed vs. not allowed

### Mixin Inclusion (Type Completion Pass Enhancement)

Mixin classes require special handling in the type completion pass, different from regular inheritance:

**Key Differences:**
1. **Methods**: Mixin methods **override** inherited base class methods (opposite of normal inheritance priority)
2. **Properties**: Mixin properties are only copied if NOT already inherited from a base class
3. **Interfaces**: Mixin's declared interfaces should be added to the including class
4. **No duplication**: Properties/methods already explicitly declared in the including class are not duplicated

This is a significant enhancement to Task 41b and may warrant a separate task (e.g., Task 41d: Mixin Inclusion Support).

### Method Override Validation (Additional Scope)

**Final method override check** should happen during compilation pass (not registration):
- When compiling a method, check if it overrides a base method
- If base method is marked `final`, produce error
- This requires comparing method signatures (name + parameters)

**Abstract/Interface method implementation check**:
- After type completion, verify all non-abstract classes implement all required methods
- Could be done as a separate validation pass after type completion
- Check that interface methods are all implemented
- Check that abstract methods from base classes are all implemented

These are more complex validations that may warrant separate tasks.

## Priority

**Phase 1 (This Task - Registration Validation):** High Priority
- Script-to-FFI class validation
- Final class inheritance validation
- Mixin cannot inherit from classes validation

**Phase 2 (Compilation Validation):** Medium Priority
- Mixin instantiation prevention
- Final method override validation
- Abstract/interface implementation validation

**Phase 3 (Type Completion Enhancement):** Medium Priority
- Mixin inclusion logic (may be Task 41d)
  - Method override behavior
  - Property merging rules
  - Interface propagation

The basic inheritance validation rules should be fixed first (Phase 1), then compilation-time checks (Phase 2), then mixin inclusion logic (Phase 3).
