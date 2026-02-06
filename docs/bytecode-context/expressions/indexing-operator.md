# Indexing Operator

## Overview
The indexing operator `[]` accesses an element within a container or object by index. The type of the index expression depends on the container type. For user-defined types, the indexing operator is implemented via the `opIndex` method.

## Syntax
```angelscript
container[index]

// Read
value = arr[0];

// Write
arr[0] = value;

// Nested
matrix[row][col] = 1;
```

## Semantics
- **Container:** Must be an expression that evaluates to a type supporting the index operator (arrays, dictionaries, strings, or any type with `opIndex` defined).
- **Index type:** Depends on the container type. Arrays use integer indices; dictionaries may use string keys; custom types define their own index type.
- **Read access:** Returns the value at the given index. For built-in arrays, out-of-bounds access throws a runtime exception.
- **Write access:** Assigns a value to the element at the given index.
- **Lvalue:** The result of indexing is an lvalue -- it can appear on the left side of an assignment.
- **Operator precedence:** Level 1 (highest), left-to-right associative. Binds tighter than all other operators.

## Examples
```angelscript
// Array indexing
array<int> arr = {10, 20, 30, 40};
int val = arr[2];       // 30
arr[0] = 99;            // arr is now {99, 20, 30, 40}

// String indexing
string s = "hello";
int ch = s[0];          // character code of 'h'

// Dictionary indexing
dictionary d;
d["key"] = 42;
int v = int(d["key"]);

// Nested indexing
array<array<int>> grid = {{1,2},{3,4}};
int cell = grid[0][1];  // 2

// Combined with increment
arr[i++] = 5;           // assigns to arr[i], then increments i
```

## Compilation Notes
- **Stack behavior:**
  - The container expression is evaluated first, pushing the object reference onto the stack.
  - The index expression is evaluated and pushed.
  - For read access: the `opIndex` call (or built-in array access) pops the index and object, and pushes the element value (or a reference to it).
  - For write access: the `opIndex` call returns a reference (lvalue) to the element, which is then used as the target of a store instruction.
- **Type considerations:**
  - The compiler must resolve the index type based on the container's `opIndex` signature.
  - Some types provide separate `opIndex` (for read/write returning a reference) methods. The compiler selects the appropriate overload based on context (read vs write).
  - The index expression may require implicit conversion to match the expected index parameter type.
- **Control flow:** No branching for the operator itself. However, bounds checking may generate a runtime exception.
- **Special cases:**
  - Out-of-bounds array access results in a runtime exception (not a compile-time error, since the index is typically not known at compile time).
  - For constant indices, the compiler could potentially emit bounds checks at compile time if the array size is known.
  - Chained indexing (`a[i][j]`) is compiled as two successive index operations, with the result of the first serving as the container for the second.
  - When the indexing result is used as an lvalue (assignment target), the compiler must ensure it emits a reference rather than a value copy.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/expr.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Expr::Index` | Index expression variant | Wraps `&IndexExpr` |
| `IndexExpr` | Array/object indexing | `object: &Expr`, `indices: &[IndexItem]`, `span: Span` |
| `IndexItem` | Single index item | `name: Option<Ident>`, `index: &Expr`, `span: Span` |

**Notes:**
- `IndexExpr` supports multiple indices via `indices: &[IndexItem]`, enabling multi-dimensional access (e.g., `matrix[row, col]`).
- Each `IndexItem` has an optional `name` field for named/associative indexing (e.g., dictionary-style `d[key: value]`).
- Chained indexing (`a[i][j]`) is represented as nested `Expr::Index` nodes, not as a single node with multiple index lists.
- `Expr::Index` is parsed as a postfix operation and shares the highest precedence tier with member access and function calls.

## Related Features
- [member-access.md](member-access.md) - Dot operator for named member access
- [increment-operators.md](increment-operators.md) - Common use of `++` in index expressions
- [assignments.md](assignments.md) - Assigning to indexed elements
