# Variable Declarations

## Overview
Variable declarations introduce named storage within a statement block. Variables are scoped to their enclosing block and must be declared before use. AngelScript supports declaring multiple variables of the same type on a single line, optional initialization expressions, and `const` qualifiers.

## Syntax
```angelscript
// Single variable, uninitialized
int x;

// Single variable with initializer
int x = 42;

// Multiple variables of the same type
int var = 0, var2 = 10;

// Handle declarations (default to null)
object@ handle, handle2;

// Constant variable (must be initialized)
const float pi = 3.141592f;

// Object variable (default-constructed)
MyClass obj;
```

## Semantics
- Variables must be declared before they are referenced within the current statement block or any nested sub-blocks.
- When execution exits the statement block where a variable was declared, that variable is no longer valid and its storage can be reclaimed.
- A variable can be declared with or without an initial expression. If an initializer is present, the expression must evaluate to a type compatible with the declared variable type.
- Any number of variables can be declared on the same line, separated by commas. All variables in such a declaration share the same type.
- Variables declared as `const` cannot be modified after initialization.
- **Default values when no initializer is provided:**
  - **Primitive types** (int, float, bool, etc.): value is **undefined** (random/uninitialized). The compiler does not zero-initialize primitives.
  - **Handle types** (`@`): initialized to `null`.
  - **Object types**: initialized via the type's default constructor.

## Examples
```angelscript
void example()
{
    int a;                    // undefined value (primitive, no initializer)
    int b = 5;                // initialized to 5
    int c = b + 1, d = b * 2; // multiple declarations: c=6, d=10
    const int MAX = 100;      // constant, cannot be reassigned

    string name;              // default-constructed (empty string)
    MyClass@ ref;             // null handle

    // a is still undefined here; using it is dangerous
    b = a;                    // legal but produces unpredictable result
}
```

## Compilation Notes
- **Variable allocation:** Each variable declaration allocates a slot in the local variable table (stack frame). The compiler assigns each variable an index/offset within the current function's stack frame. Multiple declarations on one line each get their own slot.
- **Initialization:** If an initializer expression is present, the compiler emits bytecode to evaluate the expression and store the result into the variable's slot. For primitives without initializers, no initialization bytecode is emitted (the slot contains whatever was in memory). For handles, the compiler must emit a null-store. For objects, the compiler must emit a call to the default constructor.
- **Scope tracking:** The compiler must track the start and end of each variable's scope (tied to the enclosing statement block). When the block exits, the compiler may need to emit destructor calls for object types and release calls for handles. Primitive variables need no cleanup.
- **Const enforcement:** The `const` qualifier is enforced at compile time. The compiler must reject any assignment to a `const` variable after its initialization. No special runtime bytecode is needed.
- **Multiple declarations:** `int a = 1, b = 2;` is syntactic sugar. The compiler processes each declarator independently, allocating separate slots and emitting separate initialization sequences.
- **Stack behavior:** Variable slots are typically pre-allocated when the function is entered (or when the block is entered, depending on implementation). Object destructors must be called in reverse declaration order when the block scope ends.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/stmt.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Stmt::VarDecl` | Variable declaration statement variant | Wraps `VarDeclStmt` (by value) |
| `VarDeclStmt` | Variable declaration structure | `ty: TypeExpr<'ast>`, `vars: &'ast [VarDeclarator<'ast>]`, `span: Span` |
| `VarDeclarator` | Single variable within a declaration | `name: Ident<'ast>`, `init: Option<&'ast Expr<'ast>>`, `span: Span` |

**Notes:**
- Multiple declarations on one line (`int a = 1, b = 2;`) are represented as a single `VarDeclStmt` with multiple entries in the `vars` slice.
- Each `VarDeclarator` has its own optional initializer expression.
- The shared type for all declarators is stored once in `VarDeclStmt.ty`.

## Related Features
- [Statement Blocks](./statement-blocks.md) - scoping rules that govern variable lifetime
- [Expression Statement](./expression-statement.md) - assignment expressions that modify variables
- [For Loop](./for-loop.md) - variables declared in the for-init section are scoped to the loop
