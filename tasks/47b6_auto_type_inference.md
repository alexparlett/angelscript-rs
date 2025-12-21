# Task 47b6: Auto Type Inference for Globals

## Problem

`auto` type at file/global scope fails because there's no inference context.

**Error:** `auto type cannot be resolved without inference context`

**Affected Test:** `test_types`

## Root Cause

When declaring:
```angelscript
auto inferredInt = 42;
auto inferredString = "hello";
```

At global scope, the type resolver encounters `auto` but can't infer the type because:
1. Globals are registered before function bodies are compiled
2. The initializer expression hasn't been analyzed yet

## Context

For local variables:
```angelscript
void foo() {
    auto x = 42;  // Compiler sees initializer, infers int
}
```

The compiler analyzes the initializer expression and uses its type.

For globals, the current two-pass approach:
1. Pass 1: Register types and signatures (doesn't evaluate initializers)
2. Pass 2: Compile function bodies

Globals with `auto` need their initializer analyzed to determine the type.

## Solution

### Option A: Analyze global initializers in Pass 1

When registering a global variable with `auto` type, immediately analyze the initializer expression to determine the type:

```rust
fn register_global_var(&mut self, var: &VarDecl) -> Result<()> {
    let data_type = if var.type_expr.is_auto() {
        // Must have initializer for auto globals
        let init = var.initializer.as_ref().ok_or_else(|| {
            CompilationError::Other {
                message: "auto global requires initializer".to_string(),
                span: var.span,
            }
        })?;

        // Infer type from initializer (expression-only analysis, no codegen)
        self.infer_expression_type(init)?
    } else {
        self.resolve_type(&var.type_expr)?
    };

    self.register_global(var.name, data_type);
    Ok(())
}
```

### Option B: Defer global auto types

Create a third pass or sub-pass that:
1. Collects all `auto` globals
2. Analyzes their initializers to determine types
3. Updates the global registry with resolved types

### Option C: Disallow auto for globals

If the complexity isn't worth it, emit a clear error:
```rust
if var.type_expr.is_auto() && is_global_scope {
    return Err(CompilationError::Other {
        message: "auto type is not supported for global variables; use explicit type".to_string(),
        span: var.span,
    });
}
```

## Files to Modify

- `crates/angelscript-compiler/src/passes/registration.rs` - Global variable registration
- `crates/angelscript-compiler/src/type_resolver.rs` - May need expression type inference

## Test Case

```angelscript
// These should work:
auto globalInt = 42;
auto globalString = "hello";
auto globalFloat = 3.14;

void test() {
    // Local auto already works
    auto localInt = 100;
}
```

## Acceptance Criteria

- [ ] `cargo test --test unit test_types` passes (auto portion)
- [ ] Global `auto` variables with literal initializers work
- [ ] Global `auto` variables with expression initializers work
- [ ] Proper error if `auto` global has no initializer
- [ ] No regression in local `auto` variables
