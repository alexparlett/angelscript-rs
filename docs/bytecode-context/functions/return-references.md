# Return References

## Overview
Functions in AngelScript can return references to existing values rather than copies. This allows the caller to read or modify the original value through the returned reference. To guarantee memory safety, AngelScript imposes strict rules about what a function may return by reference: the referenced value must outlive the function call.

## Syntax

### Mutable return reference
```angelscript
int property;
int &GetProperty() {
    return property;
}

void main() {
    GetProperty() = 42;  // Modifies 'property' through the reference
}
```

### Const return reference
```angelscript
const int &GetReadOnly() {
    return property;
}

void main() {
    int val = GetReadOnly();   // OK: read the value
    // GetReadOnly() = 10;     // Error: cannot assign to const reference
}
```

### Class method returning a reference to a member
```angelscript
class Container {
    int value;

    int &GetValue() {
        return value;
    }

    const int &GetValue() const {
        return value;
    }
}
```

## Semantics

### What can be returned by reference

**References to global variables are allowed.** Global variables live for the duration of the script module, which is always longer than any function call. A function may return a reference to a global variable or to a member of an object reachable through a global variable.

**References to class members are allowed (from methods).** A class method can return a reference to a property of the same object (`this`). Because the caller must hold a handle or reference to the object in order to call the method, the object (and hence its members) will remain alive after the method returns. A class method may also return a reference to a global variable, just like a free function.

### What cannot be returned by reference

**Local variables cannot be returned by reference.** Local variables are destroyed when the function exits. Returning a reference to one would create a dangling reference. The same applies to function parameters -- they are cleaned up on function exit and cannot be returned by reference.

**Expressions with deferred parameter evaluation.** When a function call has arguments that require post-call processing (e.g., cleaning up temporary input objects or copying back output parameters), the returned reference from that inner function cannot be safely returned from the outer function. The deferred cleanup could invalidate the reference.

**Expressions that rely on local objects.** All local objects must be cleaned up before a function returns. For functions returning references, this cleanup happens **before** the return expression is evaluated. Therefore, expressions that depend on local objects for evaluating the reference are not permitted. Primitive values can still be used in the return expression, since primitives do not require destructor-style cleanup.

### Const references
Adding `const` before the return type makes the returned reference read-only. The caller can read the value but cannot assign to it or pass it to a function expecting a mutable reference.

## Examples

### Global variable accessor
```angelscript
int g_score = 0;

int &Score() {
    return g_score;
}

void main() {
    Score() = 100;           // Set score to 100
    Score() += 50;           // Increment score by 50
    int current = Score();   // Read current score (150)
}
```

### Array-like index operator
```angelscript
class IntArray {
    int[] data;

    int &opIndex(uint idx) {
        return data[idx];
    }

    const int &opIndex(uint idx) const {
        return data[idx];
    }
}

void main() {
    IntArray arr;
    arr[0] = 42;             // Writes through mutable reference
    int val = arr[0];        // Reads through const or mutable reference
}
```

### Illegal: returning local variable
```angelscript
// COMPILE ERROR: cannot return reference to local variable
int &Bad() {
    int local = 10;
    return local;   // Error!
}
```

## Compilation Notes
- **Reference return mechanics:** When a function returns a reference, the bytecode places the address (pointer) of the target value into the return register, rather than copying the value itself. The caller receives this address and can use it for both reading and writing.
- **Lifetime validation:** The compiler performs static analysis at compile time to verify that the returned reference will remain valid. It checks that the referenced value is either a global variable, a class member accessed through `this`, or another expression whose lifetime exceeds the function scope. This is a compile-time check, not a runtime check.
- **Local cleanup ordering:** For reference-returning functions, the compiler schedules local variable cleanup to occur **before** the return expression is evaluated. This ensures that if a local object's destructor has side effects, they cannot invalidate the reference. As a consequence, the return expression must not depend on any local objects (but may use primitive locals, which have no cleanup).
- **Deferred argument handling:** When a call expression has deferred operations (output parameter copy-back, temporary cleanup), the compiler tracks that the returned reference may be invalidated by those deferred operations. If such a reference is used as the return value of an outer function, the compiler rejects it.
- **Const propagation:** A `const` return reference emits the same address-returning bytecode, but the compiler enforces at all use sites that no write operations target that address.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/decl.rs`, `crates/angelscript-parser/src/ast/types.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `FunctionDecl` | Function that may return a reference | `return_type: Option<ReturnType>`, plus other fields |
| `ReturnType` | Return type with reference flag | `ty: TypeExpr`, `is_ref: bool`, `span: Span` |

**Notes:**
- A function returning a reference is indicated by `ReturnType.is_ref == true`. This maps to the `&` in the return type syntax (e.g., `int &GetValue()`).
- Const return references are expressed through the `TypeExpr` within `ReturnType` (the `const` qualifier on the type), combined with `is_ref == true`.
- Lifetime validation of return references is a semantic check performed during compilation, not represented in the AST.

## Related Features
- [Function Declarations](./function-declarations.md)
- [Function Overloading](./function-overloading.md)
