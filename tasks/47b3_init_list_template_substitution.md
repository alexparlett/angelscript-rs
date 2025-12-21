# Task 47b3: Init List Template Parameter Substitution

## Problem

Init lists for template types fail because template parameters aren't substituted with concrete types.

**Error:** `TypeMismatch { message: "expected 'T', got 'int'" }`

**Affected Tests:** `test_large_function`, performance tests

## Root Cause

In `crates/angelscript-compiler/src/expr/init_list.rs:78-83`:

```rust
ListPattern::Repeat(elem_type_hash) => {
    let elem_type = DataType::simple(*elem_type_hash);  // elem_type_hash is 'T', not 'int'
    for element in expr.elements.iter() {
        compile_element(compiler, element, &elem_type, span)?;
    }
}
```

When `array<int>` is instantiated, its `ListPattern::Repeat(T)` should become `ListPattern::Repeat(int)`, but the pattern still contains the unsubstituted template parameter `T`.

## Context

For `array<int> arr = {1, 2, 3}`:
1. `array<int>` is instantiated from `array<T>`
2. The instantiated type should have `ListPattern::Repeat(int32)`
3. But it still has `ListPattern::Repeat(T)` from the template definition

## Solution

When looking up the list behavior for an instantiated template, the pattern's type hashes need to be substituted with the actual template arguments.

### Option A: Substitute during list behavior lookup

```rust
fn get_list_behavior(
    compiler: &ExprCompiler<'_, '_>,
    type_hash: TypeHash,
    span: Span,
) -> Result<ListBehavior> {
    // ... get class ...

    let mut behavior = class.behaviors.list_behaviors().first().cloned().ok_or_else(|| ...)?;

    // If this is an instantiated template, substitute pattern types
    if !class.template_args.is_empty() {
        behavior.pattern = substitute_pattern(&behavior.pattern, &class.template_params, &class.template_args);
    }

    Ok(behavior)
}

fn substitute_pattern(pattern: &ListPattern, params: &[TypeHash], args: &[TypeHash]) -> ListPattern {
    match pattern {
        ListPattern::Repeat(elem) => {
            ListPattern::Repeat(substitute_type(*elem, params, args))
        }
        ListPattern::RepeatTuple(types) => {
            ListPattern::RepeatTuple(types.iter().map(|t| substitute_type(*t, params, args)).collect())
        }
        ListPattern::Fixed(types) => {
            ListPattern::Fixed(types.iter().map(|t| substitute_type(*t, params, args)).collect())
        }
    }
}

fn substitute_type(type_hash: TypeHash, params: &[TypeHash], args: &[TypeHash]) -> TypeHash {
    // If type_hash matches a template param, return the corresponding arg
    params.iter().zip(args).find(|(p, _)| **p == type_hash).map(|(_, a)| *a).unwrap_or(type_hash)
}
```

### Option B: Substitute during template instantiation

In template instantiation code, substitute the list behavior patterns when creating the instantiated class.

## Files to Modify

- `crates/angelscript-compiler/src/expr/init_list.rs` - `get_list_behavior()` and pattern usage
- Possibly `crates/angelscript-compiler/src/template/instantiation.rs` - if fixing at instantiation time

## Test Case

```angelscript
void test() {
    array<int> arr = {1, 2, 3, 4, 5};
    array<string> strs = {"a", "b", "c"};
}
```

## Acceptance Criteria

- [ ] `cargo test --test unit test_large_function` passes
- [ ] `array<int> = {1, 2, 3}` compiles successfully
- [ ] Nested templates work: `array<array<int>> = {{1, 2}, {3, 4}}`
- [ ] Dictionary and other template types with list patterns work
- [ ] No regression in non-template init lists
