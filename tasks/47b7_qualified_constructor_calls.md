# Task 47b7: Qualified Constructor Calls

## Problem

Constructor calls with qualified type names like `Namespace::TypeName(args)` fail.

**Error:** `UnknownFunction { name: "Entity" }`

**Affected Test:** `test_nested`

## Root Cause

When compiling `Game::Entity entity(1, "Player")`:

1. The parser sees `Game::Entity` as a scoped identifier
2. The compiler tries to resolve `Entity` as a function/constructor
3. But it should resolve `Game::Entity` as a type and call its constructor

The call resolution in `crates/angelscript-compiler/src/expr/calls.rs` handles unqualified type names but not qualified paths.

## Context

```angelscript
namespace Game {
    class Entity {
        Entity(int id, string name) { ... }
    }
}

void test() {
    Game::Entity entity(1, "Player");  // Qualified constructor call
}
```

## Solution

In `compile_ident_call()`, the scoped identifier path needs to be resolved as a type first:

```rust
fn compile_ident_call<'ast>(
    compiler: &mut ExprCompiler<'_, '_>,
    ident: &IdentExpr<'ast>,
    call: &CallExpr<'ast>,
) -> Result<ExprInfo> {
    // Build qualified name from scope + ident
    let qualified_name = build_qualified_name(ident.scope, ident.ident.name);

    // Check for super() first
    if qualified_name == "super" {
        return compile_super_call(compiler, call);
    }

    // Handle template type arguments
    if !ident.type_args.is_empty() {
        // ... existing template handling ...
    }

    // Try as a type (constructor call) - use qualified name
    if let Some(type_hash) = compiler.ctx().resolve_type(&qualified_name) {
        return compile_constructor_call(compiler, type_hash, call);
    }

    // Otherwise, try as a function call
    if let Some(candidates) = compiler.ctx().resolve_function(&qualified_name) {
        // ... existing function call handling ...
    }

    // ... rest of function ...
}

fn build_qualified_name(scope: &[&str], name: &str) -> String {
    if scope.is_empty() {
        name.to_string()
    } else {
        format!("{}::{}", scope.join("::"), name)
    }
}
```

## Files to Modify

- `crates/angelscript-compiler/src/expr/calls.rs` - `compile_ident_call()` function

## Test Case

```angelscript
namespace Game {
    class Entity {
        int id;
        string name;

        Entity(int i, string n) {
            id = i;
            name = n;
        }
    }

    namespace Physics {
        class Body {
            Body(float x, float y) { }
        }
    }
}

void test() {
    Game::Entity e(1, "Player");
    Game::Physics::Body body(0.0, 0.0);
}
```

## Acceptance Criteria

- [ ] `cargo test --test unit test_nested` passes
- [ ] `Namespace::Type(args)` constructor calls work
- [ ] `Namespace::SubNamespace::Type(args)` works (deeply nested)
- [ ] Template qualified constructors work: `Namespace::Container<int>()`
- [ ] No regression in unqualified constructor calls
