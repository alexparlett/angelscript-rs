# Compiler Implementation Gaps

This document tracks identified gaps in the compiler implementation that need to be addressed.

## Status Summary

| Feature | Status | Priority | Notes |
|---------|--------|----------|-------|
| Try/Catch | ✅ Complete | - | Implemented in task-45b |
| Function Compilation | ✅ Complete | - | Implemented in task-46 |
| Reference Return Validation | ✅ Complete | - | ValueSource tracking added |
| Class Member Initialization | ❌ Missing | High | Field initializers not compiled |
| Assignment Expressions | ❌ STUBBED | Critical | Returns error at expr/mod.rs:97 |
| Import Declarations | ❌ Missing | Medium | Parser complete, not compiled |
| Global Init Order | ✅ Complete | - | Preserved in global_inits vector |

## 1. Class Member Initialization Order

**Status:** NOT IMPLEMENTED
**Priority:** HIGH
**Doc Reference:** https://www.angelcode.com/angelscript/sdk/docs/manual/doc_script_class_memberinit.html

### Problem

Per AngelScript semantics:
- Class members should be initialized in **declaration order**
- Field initializers (`int x = 5;`) should run in constructors
- Custom constructors should also initialize fields in declaration order

### Current State

- Parser supports `FieldDecl.init` (optional initializer expression)
- `registration.rs:visit_field()` ignores `field.init` completely
- `compilation.rs:compile_class()` comment says "Fields and virtual properties have no bytecode"

### Required Changes

1. **In Compilation Pass:**
   - Collect field initializers in declaration order
   - Generate initialization bytecode for constructors
   - Insert field init code at start of every constructor body

2. **Bytecode Layout for Constructor:**
   ```
   [field1 initializer]
   SetMember field1
   [field2 initializer]
   SetMember field2
   ...
   [user constructor body]
   ```

3. **Files to modify:**
   - `crates/angelscript-compiler/src/passes/compilation.rs` - compile field initializers
   - Potentially create `field_initializer.rs` module

---

## 2. Assignment Expressions

**Status:** ❌ STUBBED OUT
**Priority:** CRITICAL
**Task Reference:** task-43 (Expression Compilation - Advanced)
**Location:** `crates/angelscript-compiler/src/expr/mod.rs:97-100`

### Problem

Assignment expressions are explicitly stubbed out with an error:
```rust
Expr::Assign(_) => Err(CompilationError::Other {
    message: "Assignment not yet implemented (Task 43)".to_string(),
    span,
}),
```

### Required Functionality

1. **Simple assignment:** `a = b`
   - Validate target is lvalue
   - Check not const
   - Type check with conversions
   - Emit: `[value] SetLocal/SetGlobal/SetField`

2. **Compound assignment:** `a += b`, `a -= b`, etc.
   - Same validation as simple assignment
   - Load value, apply operation, store back
   - Emit: `GetLocal [value] Add SetLocal` (or similar)

3. **Member assignment:** `obj.field = value`
   - Need `SetField` opcode handling

4. **Index assignment:** `arr[i] = value`
   - Need to call `opIndex` setter variant

### Files to Create/Modify

- **Create:** `crates/angelscript-compiler/src/expr/assignment.rs`
- **Modify:** `crates/angelscript-compiler/src/expr/mod.rs` - dispatch to assignment module

---

## 3. Import Declarations

**Status:** NOT IMPLEMENTED
**Priority:** MEDIUM
**Doc Reference:** https://www.angelcode.com/angelscript/sdk/docs/manual/doc_global_import.html

### Problem

Import declarations allow importing functions from other modules:
```angelscript
import void func(int) from "module";
```

### Current State

- Parser fully supports `ImportDecl`
- Registration pass skips imports: `Item::Import(_) => {}`
- Compilation pass also skips imports

### Required Changes

1. **Registration Pass:**
   - Register imported function in symbol registry
   - Mark as "imported" with module name
   - No bytecode slot needed (linked at load time)

2. **Runtime Linking:**
   - When loading module, resolve imports to actual functions
   - Error if imported function not found in source module

3. **Files to modify:**
   - `crates/angelscript-compiler/src/passes/registration.rs` - register imports
   - `crates/angelscript-core/src/entries/function.rs` - add Import source type
   - Runtime/VM code for linking

---

## 4. Global Variable Initialization Order

**Status:** ✅ COMPLETE
**Doc Reference:** https://www.angelcode.com/angelscript/sdk/docs/manual/doc_global_variable.html

### Current State

- `compile_global_var()` compiles initializers
- `global_inits` vector preserves declaration order
- `GlobalInitEntry` stores bytecode for each initializer

The runtime must execute `global_inits` in order when loading the module.

---

## Implementation Order Recommendation

1. **Assignment Expressions** - CRITICAL, nothing works without this
2. **Class Member Initialization** - Important for OOP correctness
3. **Import Declarations** - Needed for multi-module programs

## Quick Reference: Key Locations

| Gap | Primary File(s) |
|-----|-----------------|
| Assignment | `expr/mod.rs:97`, create `expr/assignment.rs` |
| Field Init | `passes/compilation.rs:162`, `passes/registration.rs:364` |
| Imports | `passes/registration.rs` (Item::Import handling) |

---

## Already Completed (For Reference)

### Try/Catch (task-45b)
- `TryBegin`, `TryEnd`, `Throw` opcodes
- Exception table for catch dispatch
- Tested and working

### Function Compilation (task-46)
- `FunctionCompiler` handles setup_parameters, compile_body, verify_returns
- Implicit `this` for methods
- Reference return validation via `ValueSource`

### Reference Return Validation
- `ValueSource` enum tracks origin (Local, Global, Member, This, Temporary)
- `is_safe_for_ref_return()` prevents returning references to locals
- Tests in `return_stmt.rs`
