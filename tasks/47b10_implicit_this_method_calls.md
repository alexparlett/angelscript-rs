# Task 47b10: Implicit `this` for Method Calls

## Overview

When inside a class method, calling another method of the same class without `this.` prefix should work. Currently, `spawnEnemy(...)` inside `Game::startGame()` fails with `UnknownFunction` because the compiler doesn't check class methods when resolving bare function calls.

## Problem

```angelscript
class Game {
    void startGame() {
        spawnEnemy(10, 10, 50, 10);  // ERROR: UnknownFunction
        // Should be equivalent to:
        this.spawnEnemy(10, 10, 50, 10);  // Works
    }

    void spawnEnemy(float x, float y, int health, int damage) {
        // ...
    }
}
```

## Root Cause

In `crates/angelscript-compiler/src/expr/calls.rs`, `compile_ident_call()` resolves identifiers as:
1. `super()` call
2. Template type constructor
3. Type (constructor call)
4. Function (global/namespace function)
5. Funcdef variable

It never checks if we're inside a class method and whether the identifier matches a method of the current class.

## Solution

After checking for global functions (step 4), add a check for class methods:

```rust
// In compile_ident_call(), after the function resolution check:

// Check if we're inside a class method and this could be an implicit this.method() call
if ident.scope.is_none() {
    if let Some(class_hash) = compiler.current_class() {
        let candidates = compiler.ctx().find_methods(class_hash, name);
        if !candidates.is_empty() {
            // Push 'this' onto the stack
            compiler.emitter().emit_get_this();
            // Compile as method call
            return compile_method_call_with_candidates(
                compiler,
                &DataType::simple(class_hash),
                candidates.to_vec(),
                call.args,
                call.span,
            );
        }
    }
}
```

## Implementation Steps

1. **Modify `compile_ident_call()`** in `expr/calls.rs`:
   - After function resolution fails, check if inside a class method
   - Look up method candidates on current class
   - If found, emit `GetThis` and compile as method call

2. **Add helper or refactor `compile_method_call()`**:
   - May need to refactor to accept pre-resolved candidates
   - Or create `compile_method_call_with_candidates()` variant

3. **Handle const correctness**:
   - If current method is const, only const methods should be callable
   - Check `compiler.is_const_method()` or similar

## Files to Modify

- `crates/angelscript-compiler/src/expr/calls.rs` - Add implicit this check

## Test Cases

```angelscript
class Test {
    int value;

    void setValue(int v) { value = v; }
    int getValue() const { return value; }

    void test() {
        setValue(42);        // Should work (implicit this)
        int x = getValue();  // Should work (implicit this)
    }

    void constMethod() const {
        int x = getValue();  // Should work (const calling const)
        // setValue(1);      // Should fail (const calling non-const)
    }
}
```

## Acceptance Criteria

- [ ] `spawnEnemy(...)` inside `Game::startGame()` compiles
- [ ] Implicit this works for both const and non-const methods
- [ ] Const correctness enforced (const method can't call non-const via implicit this)
- [ ] `test_game_logic` passes
- [ ] No regression in existing tests
