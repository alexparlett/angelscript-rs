# Scope Resolution Operator

## Overview
The scope resolution operator `::` accesses variables, functions, or types from a specific scope. It is used to refer to global scope when a local variable shadows a global one, to access members of a namespace, or to call base class methods explicitly.

## Syntax
```angelscript
// Global scope (empty left-hand side)
::globalVar
::globalFunc()

// Namespace scope
NamespaceName::member
NamespaceName::func()

// Nested namespaces
Outer::Inner::item

// Base class method
BaseClass::method()
```

## Semantics
- **Global scope (`::name`):** When the left side is empty, `::` refers to the global scope. This is used to access a global variable or function that is shadowed by a local declaration with the same name.
- **Namespace scope (`NS::name`):** Accesses a member (variable, function, class, enum) within a named namespace.
- **Nested namespaces (`A::B::name`):** Multiple `::` operators can be chained to traverse nested namespace hierarchies.
- **Base class access (`Base::method()`):** In a derived class, explicitly calls a method from the base class, bypassing virtual dispatch.
- **Result:** The result depends on what is being accessed: a variable produces an lvalue, a function call produces its return value, a type produces a type reference (for use in declarations or casts).
- **Operator precedence:** Highest precedence among unary operators (top of the unary precedence list). Evaluated before all other operators.

## Examples
```angelscript
// Shadowed global variable
int value = 100;

void function() {
    int value = 42;        // shadows global
    ::value = value;       // assigns local (42) to global 'value'
    int sum = value + ::value;  // 42 + 42 = 84
}

// Namespace access
namespace Math {
    float PI = 3.14159f;
    float abs(float x) { return x >= 0 ? x : -x; }
}

float circumference = 2.0f * Math::PI * radius;
float positive = Math::abs(-5.0f);

// Nested namespaces
namespace Game {
    namespace Physics {
        void simulate() { }
    }
}

Game::Physics::simulate();

// Base class method call
class Base {
    void doWork() { /* base implementation */ }
}

class Derived : Base {
    void doWork() override {
        Base::doWork();    // call base version explicitly
        // additional work
    }
}
```

## Compilation Notes
- **Stack behavior:** The `::` operator itself does not push or pop values. It is a compile-time resolution mechanism that tells the compiler which scope to look up the identifier in. The resulting variable access, function call, or type reference is then compiled as normal.
- **Type considerations:** The `::` operator resolves to a specific symbol at compile time. The type of the expression depends on what symbol is resolved (variable type, function return type, etc.).
- **Control flow:** No branching involved. This is purely a name resolution mechanism.
- **Special cases:**
  - **Global scope prefix (`::`):** The compiler must maintain a distinction between local scope lookup and explicit global scope lookup. When `::` appears with an empty left side, the compiler skips local and enclosing scopes and searches only the global scope.
  - **Namespace resolution:** The compiler resolves the namespace chain at compile time, walking the namespace hierarchy. If the namespace or member does not exist, it is a compile-time error.
  - **Base class dispatch (`Base::method()`):** When used to call a base class method, the compiler must emit a direct (non-virtual) call to the base class method rather than going through the virtual dispatch table. This is important for avoiding infinite recursion when the derived class overrides the method.
  - **Enum values:** Enum members can be accessed via scope resolution when they would otherwise be ambiguous (e.g., `MyEnum::VALUE`).

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/expr.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Expr::Ident` | Identifier expression variant | Wraps `IdentExpr` |
| `IdentExpr` | Identifier with optional scope | `scope: Option<Scope>`, `ident: Ident`, `type_args: &[TypeExpr]`, `span: Span` |

**Notes:**
- Scope resolution (`::`) is not a separate expression node. Instead, it is encoded in the `scope: Option<Scope>` field of `IdentExpr`. A `Some(Scope)` value represents a scoped lookup (e.g., `Namespace::name` or `::globalName`).
- The `type_args` field on `IdentExpr` supports generic type arguments (e.g., `array<int>`), which may follow a scoped identifier.
- There is no dedicated `Expr::Ident` documentation file. `Expr::Ident` / `IdentExpr` serves as the general identifier reference node across the AST, used for variable references, scoped lookups, and type references.

## Related Features
- [member-access.md](member-access.md) - `.` operator for object member access
- [function-calls.md](function-calls.md) - Calling scoped functions
- [type-conversions.md](type-conversions.md) - Using scoped type names in casts
