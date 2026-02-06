# Member Access

## Overview
The dot operator (`.`) accesses members of an object -- properties (member variables) and methods (member functions). In AngelScript, member access works the same whether the expression is an object instance or an object handle.

## Syntax
```angelscript
object.property
object.method()
object.method(args)
```

## Semantics
- **Object expression:** The left-hand side must evaluate to a data type that has members (a class, interface, or registered application type). This can be a local variable, a handle, a function return value, or any expression producing an object.
- **Property access:** `object.property` reads or writes a member variable. The result is an lvalue (can be assigned to).
- **Method access:** `object.method(args)` calls a member method on the object. The return value (if any) is the expression result.
- **Handle transparency:** When a handle references an object, the members are accessed through the handle using `.` exactly as if accessing the object directly. There is no separate dereference operator.
- **Null handle access:** Accessing a member through a null handle throws a runtime exception.
- **Chained access:** Member access can be chained: `a.b.c` evaluates left-to-right, where each `.` produces an intermediate object for the next access.
- **Operator precedence:** Level 1 (post-fix unary), left-to-right associative. Among the highest-precedence operators.

## Examples
```angelscript
class Vec2 {
    float x;
    float y;

    float length() const {
        return sqrt(x * x + y * y);
    }
}

Vec2 v;
v.x = 3.0f;              // write property
v.y = 4.0f;
float len = v.length();   // call method: 5.0

// Handle access (identical syntax)
Vec2@ h = @v;
h.x = 10.0f;              // accesses v.x through handle
float hlen = h.length();  // calls method on v through handle

// Chained access
class Container {
    Vec2 position;
}

Container c;
c.position.x = 1.0f;      // chained member access

// Method returning object
Vec2 getOrigin() { return Vec2(); }
float ox = getOrigin().x;  // access property on return value
```

## Compilation Notes
- **Stack behavior:**
  - The object expression is evaluated, pushing an object reference (or value) onto the stack.
  - For property read: the member offset is used to load the property value from the object, replacing the object reference on the stack with the property value.
  - For property write: the object reference and the new value are both on the stack; a store instruction writes to the member offset.
  - For method call: the object reference becomes the `this` pointer. Arguments are pushed, then the method call instruction executes.
- **Type considerations:**
  - The compiler resolves the member name against the object's type at compile time to determine the member offset (for properties) or method signature (for methods).
  - If the object is a handle, the compiler must first dereference the handle to get the object pointer before accessing the member. This dereference may include a null check.
  - Property accessors (get/set methods registered as properties) are compiled as method calls rather than direct memory access.
- **Control flow:** No branching for the access itself. A null-handle check may be inserted as a guard before the access.
- **Special cases:**
  - **Null handle:** The runtime must detect null dereference and throw an exception. The bytecode generator may emit an explicit null check or rely on hardware traps.
  - **Virtual method dispatch:** For polymorphic types, method calls through handles or base type references require virtual dispatch (vtable lookup) rather than direct call.
  - **Const correctness:** Accessing a mutable member or calling a non-const method through a const handle is a compile-time error.
  - **Property accessors:** Types can define get/set accessor functions that are invoked transparently as if they were direct property access. The compiler must detect these and emit method calls instead of direct loads/stores.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/expr.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Expr::Member` | Member access expression variant | Wraps `&MemberExpr` |
| `MemberExpr` | Dot-operator access | `object: &Expr`, `member: MemberAccess`, `span: Span` |
| `MemberAccess::Field` | Field access (`obj.field`) | Wraps `Ident` |
| `MemberAccess::Method` | Method call (`obj.method(args)`) | `name: Ident`, `args: &[Argument]` |
| `Argument` | Function/method call argument | `name: Option<Ident>`, `value: &Expr`, `span: Span` |

**Notes:**
- The AST distinguishes field access from method calls at parse time via the `MemberAccess` enum. A `.name(` sequence produces `MemberAccess::Method`; otherwise it is `MemberAccess::Field`.
- Method calls within `MemberAccess::Method` reuse the same `Argument` struct used by standalone `CallExpr`, supporting named arguments.
- `Expr::Member` is parsed as a postfix operation and shares the highest precedence tier with indexing and function calls.
- Chained member access (`a.b.c`) is represented as nested `Expr::Member` nodes.

## Related Features
- [function-calls.md](function-calls.md) - Method call argument handling
- [handle-of.md](handle-of.md) - Handle semantics and null handles
- [scope-resolution.md](scope-resolution.md) - `::` for namespace/class scope access
- [indexing-operator.md](indexing-operator.md) - `[]` as alternative element access
