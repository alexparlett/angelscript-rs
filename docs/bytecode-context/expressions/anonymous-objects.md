# Anonymous Objects

## Overview
Anonymous objects are objects created inline in an expression without being declared as named variables. They are instantiated by calling a type's constructor as if it were a function, or by using initialization lists. Both reference types and value types can be created anonymously.

## Syntax
```angelscript
// Constructor call (positional arguments)
TypeName(arg1, arg2, arg3)

// No arguments
TypeName()

// Initialization list with explicit type
TypeName = {{key1, val1}, {key2, val2}}

// Initialization list with implicit type (inferred from context)
{val1, val2, val3}
```

## Semantics
- **Constructor invocation:** `TypeName(args)` creates a new instance of `TypeName` by calling its constructor with the given arguments. The resulting object is a temporary that lives for the duration of the enclosing expression.
- **Reference types:** For reference types, a new object is allocated on the heap and a handle to it is returned. The reference count starts at 1. When the temporary is no longer needed, the handle is released (and the object destroyed if no other handles exist).
- **Value types:** For value types, the object is typically constructed on the stack or in a temporary location. It is destroyed when the enclosing expression completes.
- **Initialization lists:** Some types support initialization with brace-enclosed lists. The type can be specified explicitly (`TypeName = {list}`) or inferred from context when only one candidate type supports initialization lists.
- **Usage contexts:** Anonymous objects are commonly used as:
  - Function arguments: `func(MyClass(1, 2))`
  - Right-hand side of assignments: `x = MyClass(5)`
  - Return values: `return MyClass(val)`
  - Elements in container initialization

## Examples
```angelscript
// Pass anonymous object as function argument
void process(MyClass obj) { }
process(MyClass(1, 2, 3));

// Anonymous object in assignment
MyClass result = MyClass(10, 20);

// Dictionary with initialization list
void useDictionary(dictionary d) { }
useDictionary(dictionary = {{"banana", 1}, {"apple", 2}, {"orange", 3}});

// Implicit type from context
void processArray(array<int> arr) { }
processArray({1, 2, 3, 4});

// Anonymous handle
MyClass@ ref = MyClass(42);

// In expressions
float len = Vec2(3.0f, 4.0f).length();
```

## Compilation Notes
- **Stack behavior:**
  - For value types: space is allocated on the stack (or in a temporary). The constructor is called with the allocated space as `this` and the provided arguments. The resulting object remains on the stack for use by the enclosing expression.
  - For reference types: a heap allocation is performed, the constructor is called, and a handle (pointer) to the new object is pushed onto the stack. Reference count is initialized to 1.
  - After the enclosing expression completes, temporary value types are destructed and temporary handles are released.
- **Type considerations:**
  - The compiler resolves the type name and selects the appropriate constructor overload based on the provided arguments.
  - For initialization lists, the compiler must identify which type supports the list syntax and match element types accordingly.
  - When the type is inferred (implicit initialization list), the compiler uses the expected type from the surrounding context (e.g., function parameter type) to determine what to construct.
- **Control flow:** No special branching. Construction follows the normal function call pattern.
- **Special cases:**
  - **Temporary lifetime:** Anonymous objects are temporaries. The compiler must ensure they are not destroyed before their value is consumed by the enclosing expression. For reference types, this means maintaining the reference count until the expression completes.
  - **Initialization lists:** The compiler must determine whether the type supports `opIndex` for list initialization or uses a dedicated initialization list constructor. The list elements are evaluated and stored before the constructor or initialization method is called.
  - **Implicit type deduction:** When the type is omitted from an initialization list, the compiler must search the surrounding context for a unique type that accepts initialization lists. If zero or multiple candidates exist, it is a compile-time error.
  - **Return value optimization:** When an anonymous object is directly returned from a function or used to initialize a variable, the compiler may optimize by constructing the object directly in the target location, avoiding a copy.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/expr.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Expr::Call` | Function call expression variant | Wraps `&CallExpr` |
| `CallExpr` | Function/constructor call | `callee: &Expr`, `args: &[Argument]`, `span: Span` |
| `Expr::Ident` | Identifier expression variant (type name) | Wraps `IdentExpr` |
| `IdentExpr` | Identifier with optional scope | `scope: Option<Scope>`, `ident: Ident`, `type_args: &[TypeExpr]`, `span: Span` |
| `Expr::InitList` | Initialization list expression variant | Wraps `InitListExpr` |
| `InitListExpr` | Brace-enclosed initializer list | `ty: Option<TypeExpr>`, `elements: &[InitElement]`, `span: Span` |

**Notes:**
- Anonymous object construction via `TypeName(args)` is parsed as `Expr::Call` where the `callee` is an `Expr::Ident` containing the type name. The parser does not distinguish between function calls and constructor calls at the AST level; semantic analysis resolves this.
- Initialization list syntax (`TypeName = {list}` or `{list}`) is parsed as `Expr::InitList` with an optional `ty: Option<TypeExpr>` for the explicit type annotation.
- There is no dedicated identifier expression documentation file. `Expr::Ident` / `IdentExpr` is used for both variable references and type name references across the AST.

## Related Features
- [function-calls.md](function-calls.md) - Passing anonymous objects as arguments
- [type-conversions.md](type-conversions.md) - Constructor-style explicit value casts
- [member-access.md](member-access.md) - Accessing members on anonymous objects
