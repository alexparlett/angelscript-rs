# Task 41d: Const-Correctness Implementation

**Status**: Not Started
**Dependencies**: Task 41 (Expression Compilation - Basics)
**Related**: Task 41 identified TODOs for const-correctness that need holistic implementation

## Overview

Implement const-correctness checks across the compiler to ensure that:
1. Non-const methods cannot be called on const objects
2. Const methods can be called on both const and non-const objects
3. Type conversions respect const-correctness

This is a systematic implementation of const-correctness that was identified during Task 41 but deserves its own focused task.

## Background

During Task 41 implementation, we identified that const-correctness checks were needed in three places:
1. **Operator resolution** - When resolving user-defined operators (binary/unary)
2. **Conversion resolution** - When finding conversion methods between types
3. **Overload resolution** - When selecting function overloads (future task)

Task 41 specification (lines 104-105) explicitly mentioned const-correctness for operators but left TODOs in place. This task completes that work holistically.

## Const-Correctness Rules

### Rule 1: Method Calls
- A **non-const method** cannot be called on a **const object**
- A **const method** can be called on **both const and non-const objects**

### Rule 2: DataType Const Fields
`DataType` has two const-related fields:
- `is_const: bool` - Whether the value itself is const (immutable)
- `is_handle_to_const: bool` - Whether this is a handle to a const value

### Rule 3: FunctionDef Const Check
`FunctionDef` has:
- `is_const() -> bool` - Returns whether a method is const

### Implementation Pattern
```rust
// Check if we can call this method on this object
if !func_entry.is_const() && (obj_type.is_const || obj_type.is_handle_to_const) {
    continue; // Skip this method - it requires non-const receiver but object is const
}
```

## Goals

### Goal 1: Implement Const-Correctness for Binary Operators
**File**: `crates/angelscript-compiler/src/operators/binary.rs`

**Location**: Line 199-200 (TODO comment)

**Current Code**:
```rust
// Const-correctness check
// TODO: Implement const-correctness checks for methods and parameters
```

**Required Implementation**:
```rust
// Const-correctness check
// Rule: Non-const methods cannot be called on const objects
// Const methods can be called on both const and non-const objects
if !func_entry.is_const() && (obj_type.is_const || obj_type.is_handle_to_const) {
    continue; // This method requires non-const receiver, but object is const
}
```

**Context**: This check happens when trying each operator overload (e.g., `opAdd`, `opMul`) on the left or right operand.

### Goal 2: Implement Const-Correctness for Unary Operators
**File**: `crates/angelscript-compiler/src/operators/unary.rs`

**Location**: Line 88-89 (TODO comment)

**Current Code**:
```rust
// Const-correctness check
// TODO: Implement const-correctness checks for methods
```

**Required Implementation**:
```rust
// Const-correctness check
// Rule: Non-const methods cannot be called on const objects
if !func_entry.is_const() && (operand.is_const || operand.is_handle_to_const) {
    continue; // This method requires non-const receiver, but operand is const
}
```

**Context**: This check happens when trying each operator overload (e.g., `opNeg`, `opCom`) on the operand.

### Goal 3: Implement Const-Correctness for Conversion Methods
**File**: `crates/angelscript-compiler/src/conversion/user_defined.rs`

**Current Issue**: Functions like `find_implicit_conv_method` and `find_cast_method` only take `TypeHash` parameters, so they cannot check const-correctness.

**Affected Functions**:
- `find_implicit_conv_method(source: TypeHash, target: TypeHash, ...)` (line 51)
- `find_cast_method(source: TypeHash, target: TypeHash, ...)` (line 88)

**Required Changes**:
1. Change signatures to accept `&DataType` instead of `TypeHash`
2. Add const-correctness checks when evaluating conversion methods
3. Ensure conversion methods respect const on the source object

**Example Change**:
```rust
// Before
fn find_implicit_conv_method(
    source: TypeHash,
    target: TypeHash,
    ctx: &CompilationContext<'_>,
) -> Option<TypeHash>

// After
fn find_implicit_conv_method(
    source: &DataType,
    target: &DataType,
    ctx: &CompilationContext<'_>,
) -> Option<TypeHash>
```

Then add checks:
```rust
// When checking if a conversion method is viable
if !func_entry.is_const() && (source.is_const || source.is_handle_to_const) {
    continue; // Cannot call non-const conversion method on const object
}
```

**Propagation**: This change will require updating callers in:
- `conversion/mod.rs` - `find_conversion()` function
- Any other code calling these functions

## Non-Goals

- **Overload resolution** (function call resolution) - This belongs in Task 42 when we implement function calls
- **Assignment const-correctness** - Checking that you can't assign to const lvalues (separate concern)
- **Parameter const-correctness** - Validating const parameters in function signatures (done during registration)

## Implementation Steps

### Step 1: Binary Operators ✓ (operators/binary.rs)
1. Replace TODO at line 199-200 with const-correctness check
2. Use pattern: `if !func_entry.is_const() && (obj_type.is_const || obj_type.is_handle_to_const)`
3. This applies to both primary and reverse operator lookups

### Step 2: Unary Operators ✓ (operators/unary.rs)
1. Replace TODO at line 88-89 with const-correctness check
2. Use same pattern: `if !func_entry.is_const() && (operand.is_const || operand.is_handle_to_const)`

### Step 3: Conversion Methods (conversion/user_defined.rs)
1. Change `find_implicit_conv_method` signature to accept `&DataType`
2. Change `find_cast_method` signature to accept `&DataType`
3. Add const-correctness checks in both functions
4. Update callers in `conversion/mod.rs`

### Step 4: Testing
1. Run existing tests to ensure nothing breaks: `/test`
2. Consider adding specific test cases for const-correctness (optional for this task)
3. Run clippy: `/clippy`

## Acceptance Criteria

1. ✅ All TODOs for const-correctness are removed
2. ✅ Binary operator resolution checks method const-ness against receiver
3. ✅ Unary operator resolution checks method const-ness against operand
4. ✅ Conversion method resolution checks method const-ness against source type
5. ✅ All existing tests pass
6. ✅ Clippy reports no new warnings

## Test Scenarios (Future)

While not required for this task, here are test scenarios that would validate const-correctness:

1. **Const object calling non-const operator**: Should fail to compile
   ```angelscript
   const MyType obj;
   obj + 5;  // Error if opAdd is non-const
   ```

2. **Const object calling const operator**: Should succeed
   ```angelscript
   const MyType obj;
   obj == other;  // OK if opEquals is const
   ```

3. **Const object with implicit conversion**: Should fail if conversion method is non-const
   ```angelscript
   const MyType obj;
   OtherType other = obj;  // Error if MyType's conversion method is non-const
   ```

## References

- **Task 41 Specification**: Lines 104-105 explicitly require const-correctness for operators
- **DataType Definition**: `crates/angelscript-core/src/data_type.rs` (lines 85-88, 97)
- **FunctionDef is_const()**: `crates/angelscript-core/src/function_def.rs` (line 368-370)
- **AngelScript Documentation**: Const-correctness rules from official docs

## Notes

- This task focuses on **compile-time checks** only
- The const flags in `DataType` are already set correctly by earlier passes
- We're just enforcing the rules when selecting methods/operators
- This is a pure compile-time safety feature with no runtime overhead
