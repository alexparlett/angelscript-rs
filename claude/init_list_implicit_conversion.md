# Init List Implicit Conversion

## Problem

When initializing an array with a known element type, literal values should be implicitly converted to match the target type. Currently this fails:

```angelscript
array<float> arr = { 1.0, 2.0, 3.0 };  // ERROR: array<double> type not found
```

The literals `1.0`, `2.0`, `3.0` are `double` by default. The compiler tries to infer the init list type as `array<double>`, but no such type has been instantiated. Even though `double` → `float` is a valid implicit conversion, it's not being applied.

### Workaround

Use explicit float literals:
```angelscript
array<float> arr = { 1.0f, 2.0f, 3.0f };  // OK
```

## Expected Behavior

When the target type is known (e.g., `array<float>`), init list elements should be implicitly converted:

```angelscript
array<float> arr = { 1.0, 2.0, 3.0 };     // Should work: double → float
array<int> arr = { 1, 2, 3 };              // Already works: int → int
array<double> arr = { 1.0f, 2.0f };        // Should work: float → double
array<int64> arr = { 1, 2, 3 };            // Should work: int → int64
```

## Current Implementation

In `function_processor.rs`, init lists are handled in `check_init_list()`:

1. If target type is known (from variable declaration), use it
2. Otherwise, infer type from first element
3. Check that all elements match the inferred/expected type

The issue is in step 3 - we require exact type match instead of allowing implicit conversions.

## Proposed Fix

When checking init list elements against the expected element type:
1. Check if element type matches exactly → OK
2. Check if element type can implicitly convert to expected type → OK, emit conversion
3. Otherwise → error

### Key Code Location

`src/semantic/passes/function_processor.rs` - look for `check_init_list` or init list handling in `check_expr`.

## Implementation Steps

1. Find init list handling in `function_processor.rs`
2. When comparing element types to expected type, use `can_convert_to()` instead of exact match
3. Only allow implicit conversions (not explicit casts)
4. Emit conversion instructions for each element that needs it
5. Add test cases

## Test Cases

```angelscript
// Basic numeric conversions
void testNumericConversions() {
    array<float> floats = { 1.0, 2.0, 3.0 };           // double → float
    array<double> doubles = { 1.0f, 2.0f, 3.0f };     // float → double
    array<int64> longs = { 1, 2, 3 };                  // int → int64
    array<float> mixed = { 1, 2.0, 3.0f };            // int→float, double→float, float→float
}

// Should still fail - no implicit conversion
void testExplicitOnly() {
    array<int> ints = { 1.5, 2.5 };  // ERROR: double → int requires explicit cast
}

// Nested arrays
void testNested() {
    array<array<float>> matrix = {
        { 1.0, 2.0 },    // inner arrays: double → float
        { 3.0, 4.0 }
    };
}

// Handle types
void testHandles() {
    array<Base@> bases = { derived1, derived2 };  // Derived@ → Base@ (already works?)
}
```

## Edge Cases

1. **Empty init list**: `array<float> arr = {}` - should work, no elements to convert
2. **Null in handle array**: `array<Foo@> arr = { null, foo }` - null is compatible with any handle
3. **Mixed types requiring different conversions**: `{ 1, 2.0, 3.0f }` for `array<float>`
4. **Nested init lists**: Each level needs its own conversion check

## Related

- Implicit conversion rules: `src/semantic/types/data_type.rs` - `can_convert_to()`
- Current init list handling: `src/semantic/passes/function_processor.rs`
- Similar issue: function argument implicit conversions (already working)
