# Task 41d: Mixin Class Support

## Overview

Implement full support for AngelScript mixin classes, which are special class-like constructs that get "mixed in" to other classes. This requires changes to `ClassEntry`, registration pass, type completion pass, and validation.

## Context

AngelScript mixin classes are not real types - they're templates for code that gets copied into classes that include them. Currently:
- Parser recognizes `mixin class` syntax and creates `MixinDecl` AST nodes
- Registration pass **ignores** mixin declarations (`Item::Mixin(_) => { /* deferred */ }`)
- `ClassEntry` has no `is_mixin` field to track mixin status
- Type completion pass doesn't handle mixin inclusion

**Reference**: https://angelcode.com/angelscript/sdk/docs/manual/doc_script_mixin.html

## Mixin Class Semantics

### What Mixins Are
- Not real types - cannot be instantiated
- Templates for properties and methods that get copied into including classes
- Can declare required interfaces
- Can reference members not declared in the mixin itself (resolved in including class context)

### Inheritance Rules
- **Cannot** inherit from other classes
- **Can** declare interfaces (which the including class must implement)
- **Can** be included by multiple classes (code is replicated)

### Inclusion Behavior
When `class Foo : MyMixin` includes a mixin:

1. **Methods**: Copied to including class
   - **Override base class methods** (opposite priority from normal inheritance!)
   - Not duplicated if explicitly declared in the including class

2. **Properties**: Copied to including class
   - **NOT copied if already inherited** from base class
   - Not duplicated if explicitly declared in the including class

3. **Interfaces**: Mixin's declared interfaces added to including class
   - Including class must implement all interface methods (can come from mixin or class itself)

4. **Member Resolution**: Mixin methods can reference members that don't exist in mixin
   - Resolved at compile time in the context of the including class

## Implementation Plan

### Phase 1: Core Infrastructure

**1. Add `is_mixin` to `ClassEntry`**

File: `crates/angelscript-core/src/entries/class.rs`

```rust
pub struct ClassEntry {
    // ... existing fields ...

    // === Modifiers ===
    /// Class is marked `final`.
    pub is_final: bool,
    /// Class is marked `abstract`.
    pub is_abstract: bool,
    /// Class is a mixin (not a real type).
    pub is_mixin: bool,  // NEW
}
```

Update constructor and builder methods:
```rust
impl ClassEntry {
    pub fn new(...) -> Self {
        Self {
            // ... existing fields ...
            is_mixin: false,
        }
    }

    /// Mark this class as a mixin.
    pub fn as_mixin(mut self) -> Self {
        self.is_mixin = true;
        self
    }

    /// Create a mixin class entry (script-defined).
    pub fn script_mixin(
        name: impl Into<String>,
        namespace: Vec<String>,
        qualified_name: impl Into<String>,
        source: TypeSource,
    ) -> Self {
        Self::script(name, namespace, qualified_name, source)
            .as_mixin()
    }
}
```

**2. Register Mixins in Registration Pass**

File: `crates/angelscript-compiler/src/passes/registration.rs`

Change from ignoring mixins to registering them:

```rust
fn visit_item(&mut self, item: &Item<'ast>) {
    match item {
        Item::Class(class) => {
            self.register_class(class);
        }
        Item::Mixin(mixin) => {
            // NEW: Register mixin as a special class
            self.register_mixin(&mixin.class);
        }
        // ... other cases ...
    }
}

/// Register a mixin class.
fn register_mixin(&mut self, class: &ClassDecl<'_>) {
    // Resolve inheritance (interfaces only - validated in Task 41c)
    let (base_class, interfaces) = self.resolve_inheritance(class);

    // Validation: mixins cannot have base classes (Task 41c)
    if base_class.is_some() {
        self.ctx.add_error(CompilationError::InvalidOperation {
            message: format!(
                "mixin class '{}' cannot inherit from other classes",
                class.name.name
            ),
            span: class.span,
        });
    }

    // Create class entry marked as mixin
    let class_entry = ClassEntry::script_mixin(
        class.name.name,
        Vec::new(), // TODO: namespace from context
        class.name.name,
        TypeSource::script(self.unit_id, class.span),
    )
    .with_interfaces(interfaces);

    // Register the type
    if let Err(e) = self.ctx.register_type(class_entry.into()) {
        self.ctx.add_error(e.into());
    }

    // Register members (methods, properties) - same as regular class
    self.register_class_members(class);
}
```

### Phase 2: Type Completion Enhancement

**3. Handle Mixin Inclusion in Type Completion Pass**

File: `crates/angelscript-compiler/src/passes/completion.rs`

Enhance `complete_class` to detect and handle mixin inclusion:

```rust
fn complete_class(
    &mut self,
    class_hash: TypeHash,
    output: &mut CompletionOutput,
) -> Result<(), CompilationError> {
    let class = self.registry.get(class_hash)
        .and_then(|e| e.as_class())?;

    // Skip mixin classes themselves - they don't inherit
    if class.is_mixin {
        return Ok(());
    }

    let base_hash = match class.base_class {
        Some(h) => h,
        None => return Ok(()),
    };

    let base = self.registry.get(base_hash)
        .and_then(|e| e.as_class())?;

    // Check if base is a mixin - different rules apply
    if base.is_mixin {
        self.apply_mixin(class_hash, base, output)?;
    } else {
        // Regular inheritance (existing logic)
        self.apply_inheritance(class_hash, base, output)?;
    }

    Ok(())
}

/// Apply mixin inclusion (different from regular inheritance).
fn apply_mixin(
    &mut self,
    class_hash: TypeHash,
    mixin: &ClassEntry,
    output: &mut CompletionOutput,
) -> Result<(), CompilationError> {
    // Phase 1: Collect members to copy
    let to_copy = {
        let including_class = self.registry.get(class_hash)
            .and_then(|e| e.as_class())?;

        let mut methods = Vec::new();
        let mut properties = Vec::new();

        // Copy ALL methods from mixin (public/protected/private)
        // Mixin methods OVERRIDE inherited methods from base classes
        for (name, method_hashes) in &mixin.methods {
            for &method_hash in method_hashes {
                // Skip if method is explicitly declared in including class
                if !including_class.has_method(name) {
                    methods.push((name.clone(), method_hash));
                }
            }
        }

        // Copy properties from mixin UNLESS already inherited from base
        for property in &mixin.properties {
            // Check if property already exists in including class
            // (either declared or inherited from base class)
            if !including_class.has_property(&property.name) {
                properties.push(property.clone());
            }
        }

        // Collect mixin's interfaces
        let interfaces = mixin.interfaces.clone();

        (methods, properties, interfaces)
    }; // immutable borrow ends

    // Phase 2: Apply to including class
    let class = self.registry.get_class_mut(class_hash)?;

    for (name, method_hash) in to_copy.0 {
        class.add_method(name, method_hash);
        output.methods_inherited += 1;
    }

    for property in to_copy.1 {
        class.properties.push(property);
        output.properties_inherited += 1;
    }

    // Add mixin's interfaces to including class
    for interface_hash in to_copy.2 {
        if !class.interfaces.contains(&interface_hash) {
            class.interfaces.push(interface_hash);
        }
    }

    Ok(())
}
```

**Note**: The mixin method override behavior (overriding base class methods) is complex:
- Need to track which methods came from base class vs. declared directly
- Mixin methods should replace base class methods in the method table
- This might require tracking method "origin" (declared, inherited, from mixin)

### Phase 3: Validation (Task 41c Integration)

**4. Prevent Mixin Instantiation (Compilation Pass)**

File: `crates/angelscript-compiler/src/passes/compilation.rs` (when implemented)

```rust
/// Check if a type can be instantiated.
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

Call this check when:
- Declaring variables: `MyMixin m;` ❌
- Creating objects: `MyMixin@ m = MyMixin();` ❌
- Function parameters/returns: Need to check if type is instantiable

## Testing

### Unit Tests (Registration Pass)

```rust
#[test]
fn register_mixin_class() {
    let source = r#"
        mixin class Helper {
            void helpMethod() {}
        }
    "#;
    let arena = bumpalo::Bump::new();
    let script = Parser::parse(source, &arena).unwrap();

    let registry = SymbolRegistry::with_primitives();
    let mut ctx = CompilationContext::new(&registry);
    let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
    let output = pass.run(&script);

    assert_eq!(output.errors.len(), 0);
    assert_eq!(output.types_registered, 1);

    // Verify it's registered as a mixin
    let helper_hash = ctx.resolve_type("Helper").unwrap();
    let helper = ctx.get_type(helper_hash).unwrap().as_class().unwrap();
    assert!(helper.is_mixin);
}

#[test]
fn register_mixin_cannot_inherit_from_class() {
    let source = r#"
        class Base {}
        mixin class Helper : Base {}
    "#;
    let arena = bumpalo::Bump::new();
    let script = Parser::parse(source, &arena).unwrap();

    let registry = SymbolRegistry::with_primitives();
    let mut ctx = CompilationContext::new(&registry);
    let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
    let output = pass.run(&script);

    // Should have error about mixin inheriting from class
    assert!(output.errors.len() > 0);
    assert!(output.errors.iter().any(|e| {
        matches!(e, CompilationError::InvalidOperation { message, .. }
            if message.contains("mixin") && message.contains("cannot inherit"))
    }));
}
```

### Integration Tests (Type Completion)

```rust
#[test]
fn complete_mixin_inclusion() {
    let source = r#"
        mixin class RenderMixin {
            void render() { }
        }

        class Sprite : RenderMixin {
            void update() { }
        }
    "#;
    let arena = bumpalo::Bump::new();
    let script = Parser::parse(source, &arena).unwrap();

    let registry = SymbolRegistry::with_primitives();
    let mut ctx = CompilationContext::new(&registry);

    // Registration
    let reg_pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
    let reg_output = reg_pass.run(&script);
    assert_eq!(reg_output.errors.len(), 0);

    // Type completion
    let comp_pass = TypeCompletionPass::new(&mut ctx);
    let comp_output = comp_pass.run();
    assert_eq!(comp_output.errors.len(), 0);

    // Verify Sprite has render() method from mixin
    let sprite_hash = ctx.resolve_type("Sprite").unwrap();
    let sprite = ctx.get_type(sprite_hash).unwrap().as_class().unwrap();
    assert!(sprite.has_method("render"));
    assert!(sprite.has_method("update"));
}

#[test]
fn complete_mixin_method_overrides_base() {
    let source = r#"
        class Base {
            void foo() { }
        }

        mixin class Mixin {
            void foo() { /* override */ }
        }

        class Derived : Base, Mixin {
        }
    "#;
    // After completion, Derived.foo() should be the version from Mixin, not Base
    // This is the opposite of normal inheritance priority!
}
```

## Files to Modify

1. **crates/angelscript-core/src/entries/class.rs**
   - Add `is_mixin: bool` field
   - Add `as_mixin()` and `script_mixin()` constructors

2. **crates/angelscript-compiler/src/passes/registration.rs**
   - Add `register_mixin()` method
   - Change `Item::Mixin(_) => { ... }` to call `register_mixin()`

3. **crates/angelscript-compiler/src/passes/completion.rs**
   - Add `apply_mixin()` method
   - Update `complete_class()` to detect mixin inclusion
   - Handle mixin method override priority

4. **crates/angelscript-compiler/src/passes/compilation.rs** (future)
   - Add `check_instantiable()` validation

## Acceptance Criteria

### Phase 1: Core Infrastructure
- [ ] `ClassEntry` has `is_mixin` field
- [ ] `as_mixin()` and `script_mixin()` constructors work
- [ ] Registration pass registers mixin classes with `is_mixin = true`
- [ ] Mixin inheriting from class produces error (Task 41c validation)

### Phase 2: Type Completion
- [ ] Mixin methods copied to including class
- [ ] Mixin properties copied to including class
- [ ] Mixin methods override base class methods
- [ ] Mixin properties NOT copied if already inherited from base
- [ ] Mixin interfaces added to including class
- [ ] Properties/methods already declared in class not duplicated

### Phase 3: Validation
- [ ] Cannot instantiate mixin class (compilation error)
- [ ] Clear error messages for all mixin violations

### Testing & Quality
- [ ] Unit tests for mixin registration
- [ ] Unit tests for mixin validation
- [ ] Integration tests for mixin inclusion
- [ ] Test mixin method override priority
- [ ] All existing tests still pass
- [ ] No clippy warnings

## Priority

**High Priority** - Mixins are a core AngelScript feature and blocking compilation of scripts that use them.

**Dependencies:**
- Blocked by: Nothing (parser already supports mixins)
- Blocks: Task 41c validation (needs `is_mixin` field)
- Blocks: Any script compilation using mixins

**Suggested Order:**
1. Task 41d Phase 1 (infrastructure + registration)
2. Task 41c (validation with `is_mixin` available)
3. Task 41d Phase 2 (type completion mixin inclusion)
4. Task 41d Phase 3 (compilation-time validation)

## Open Questions

1. **Method Override Priority**: How to track method origin?
   - Option A: Add `origin: MemberOrigin` enum to method storage
   - Option B: Process mixins after base class in completion, replace methods
   - Option C: Store "override priority" and resolve at lookup time

2. **Multiple Mixins**: Can a class include multiple mixins?
   - AngelScript docs suggest yes (similar to multiple inheritance for mixins)
   - May need to track mixin list separately from `base_class`

3. **Mixin Member Resolution**: How to validate mixin methods referencing non-existent members?
   - Need to compile mixin methods in context of including class
   - May require deferred compilation or special handling

## Notes

- Mixins are essentially code generation/copying, not true inheritance
- The "mixin methods override base methods" rule is CRITICAL and opposite from normal inheritance
- This affects method dispatch and needs careful handling
- Mixins can make method resolution more complex (need clear precedence rules)
