# Handle-of Operator

## Overview
The `@` (handle-of) operator obtains or manipulates object handles (references). Handles are reference-counted pointers that allow multiple variables to refer to the same object instance. The `@` operator is used to explicitly work with the handle itself rather than the value of the referenced object.

## Syntax
```angelscript
// Get handle to an object
@handle = @object;

// Clear a handle (release reference)
@handle = null;

// Declare a handle variable
Type@ handleVar;

// Pass handle explicitly
func(@object);
```

## Semantics
- **Handle-of (`@`):** When applied to an object expression, `@` produces the handle (address/reference) rather than the value. This is used to assign handles, pass handles, and compare handles.
- **Handle assignment (`@h = @o`):** Makes handle `h` refer to the same object as `o`. Increments the reference count of the target object. Decrements the reference count of the previously referenced object (if any). If the previous count reaches zero, the old object is destroyed.
- **Null assignment (`@h = null`):** Releases the handle's reference to its object. If this was the last reference, the object is destroyed.
- **Without `@`:** When used without `@`, operators on handles work on the **referenced object** instead. For example, `h = o` copies the value of `o` into the object `h` references (using `opAssign`). This requires `h` to be non-null.
- **Handle types:** Only reference types support handles. Primitives and most value types do not have handles.
- **Reference counting:** Handles use reference counting for automatic memory management. An object is destroyed when its reference count reaches zero.
- **Operator precedence:** `@` is a unary pre-operator at precedence level 2 (lower than post-operators like `.` and `[]`, but higher than binary operators).

## Examples
```angelscript
class MyClass {
    int value;
}

// Object instance and handle
MyClass obj;
obj.value = 10;

MyClass@ handle;           // null handle
@handle = @obj;            // handle now references obj
handle.value = 20;         // modifies obj.value to 20 (transparent access)

// Handle vs value assignment
MyClass obj2;
handle = obj2;             // copies obj2's value into obj (via opAssign)
@handle = @obj2;           // handle now references obj2 instead

// Null handling
@handle = null;            // release reference to obj2
// If no other handles reference obj2, it is destroyed here

// Multiple handles to same object
MyClass@ a = MyClass();
MyClass@ b = @a;           // both reference same object
a.value = 42;
// b.value is also 42

// Handle in function parameters
void process(MyClass@ ref) {
    ref.value = 100;
}
process(@obj);             // pass by handle
```

## Compilation Notes
- **Stack behavior:**
  - `@expr` pushes the object's address/handle onto the stack (not the object's value).
  - `@h = @o` compiles to: evaluate `@o` (push address), store into handle variable `h` (which involves reference count manipulation).
- **Type considerations:**
  - The compiler must distinguish between handle context (using `@`) and value context (without `@`). This affects whether assignment copies a value or rebinds a reference.
  - Handle types are internally represented as pointers. The `@` operator on the source side simply produces the pointer value; on the destination side, it indicates that the target variable should be treated as a handle rather than a value.
- **Control flow:** No branching for the handle-of operator itself.
- **Special cases:**
  - **Reference counting:** Every handle assignment requires:
    1. Increment the reference count of the new target object.
    2. Decrement the reference count of the old target object.
    3. If the old count reaches zero, invoke the destructor and deallocate.
    The bytecode generator must emit these reference count operations around every handle assignment.
  - **Null assignment:** When assigning `null`, only the decrement and potential destruction of the old target is needed.
  - **Implicit handle operations:** The compiler sometimes infers handle semantics without explicit `@`. For example, when assigning the result of a constructor to a handle variable, the `@` may be implicit.
  - **Const handles:** `const Type@` prevents modifying the referenced object through the handle. `Type@ const` prevents reassigning the handle itself. Both constraints are enforced at compile time.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/expr.rs`, `crates/angelscript-parser/src/ast/ops.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Expr::Unary` | Unary prefix expression variant | Wraps `&UnaryExpr` |
| `UnaryExpr` | Unary prefix operation | `op: UnaryOp`, `operand: &Expr`, `span: Span` |
| `UnaryOp::HandleOf` | `@` handle-of operator | binding_power: 25 |

**Notes:**
- The `@` operator is represented as a unary prefix expression in the AST, sharing the same `UnaryOp::binding_power()` of 25 with all other unary prefix operators (`Neg`, `Plus`, `LogicalNot`, `BitwiseNot`, `PreInc`, `PreDec`).
- In practice, `@` has lower precedence than postfix operators (member access, indexing, calls) at 27, so `@obj.field` parses as `@(obj.field)`.
- The semantic distinction between handle context (`@h = @o`) and value context (`h = o`) is determined during semantic analysis, not at the AST level.

## Related Features
- [identity-comparison.md](identity-comparison.md) - `is` / `!is` for comparing handle identity
- [member-access.md](member-access.md) - Accessing members through handles
- [assignments.md](assignments.md) - Value vs handle assignment distinction
- [type-conversions.md](type-conversions.md) - Reference casts between handle types
