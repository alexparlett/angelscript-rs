# Foreach Loop

## Overview
The `foreach` loop iterates over the elements of a container (such as an array or dictionary) without requiring explicit index management. It is a contextual keyword in AngelScript -- `foreach` is recognized as a keyword only when it appears at the start of a statement. The loop supports one or more iteration variables, which is useful for containers that expose key-value pairs.

## Syntax
```angelscript
// Single iteration variable
foreach (Type var : expression)
{
    // body -- var holds each element in turn
}

// Multiple iteration variables (e.g., key and value)
foreach (Type1 var1, Type2 var2 : expression)
{
    // body -- var1 and var2 are populated per element
}

// Single-statement body (no braces)
foreach (int x : items)
    process(x);
```

## Semantics
- The expression after the colon (`:`) must evaluate to a type that supports iteration (e.g., arrays, dictionaries, or any type with the appropriate opForEach / opForValue / opForKey methods).
- Each iteration variable is declared with an explicit type and name. The type must be compatible with what the container yields.
- **Single variable:** The variable receives each element value from the container.
- **Multiple variables:** For containers like dictionaries, the first variable typically receives the key and the second receives the value. The exact mapping depends on the container's iteration interface.
- Variables declared in the foreach header are scoped to the foreach statement (header and body).
- The body can be a single statement or a statement block (curly braces).
- `break` and `continue` can be used within the body. `break` exits the loop entirely; `continue` skips to the next iteration.
- The iteration order depends on the container type (sequential for arrays, implementation-defined for dictionaries).

## Examples
```angelscript
// Iterate over an array
int[] numbers = {1, 2, 3, 4, 5};
foreach (int n : numbers)
{
    print("" + n);
}

// Iterate over a dictionary (key-value)
dictionary dict;
dict["a"] = 1;
dict["b"] = 2;
foreach (string key, int value : dict)
{
    print(key + " = " + value);
}

// Using break and continue
foreach (int x : items)
{
    if (x < 0)
        continue;  // skip negative values
    if (x > 100)
        break;     // stop at first value over 100
    process(x);
}
```

## Compilation Notes
- **Desugaring:** The foreach loop is typically compiled by desugaring into a while loop that uses the container's iteration interface. The general pattern is:
  1. Evaluate the container expression once and store a reference.
  2. Obtain an iterator from the container.
  3. Enter a while loop that checks whether the iterator has more elements.
  4. Each iteration extracts the current element(s) into the declared variable(s) and advances the iterator.
  5. The body is emitted inside the loop.
- **Variable scoping:** The iteration variables are scoped to the foreach statement. The compiler creates a scope encompassing the entire foreach construct. Variables must be destructed/released when the loop exits.
- **Container interface:** The compiler must verify that the container expression's type supports iteration. This typically requires specific methods (e.g., `opForBegin`, `opForEnd`, `opForNext`, `opForValue`, or equivalent). The exact interface depends on the registered type behaviors.
- **Break/continue targets:**
  - `break` jumps to the end of the foreach loop (after cleanup).
  - `continue` jumps to the iterator advancement (next iteration check).
- **Stack behavior:** The container reference and iterator state are maintained across iterations. The iteration variables are assigned each iteration. All must be cleaned up when the loop exits.
- **Special cases:**
  - An empty container results in zero iterations (the body is never executed).
  - Object-type iteration variables must be properly constructed/destructed each iteration.
  - Handle-type iteration variables must have their reference counts managed correctly.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/stmt.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Stmt::Foreach` | Foreach loop statement variant | Wraps `&'ast ForeachStmt<'ast>` (arena-allocated reference) |
| `ForeachStmt` | Foreach loop structure | `vars: &'ast [ForeachVar<'ast>]`, `expr: &'ast Expr<'ast>`, `body: &'ast Stmt<'ast>`, `span: Span` |
| `ForeachVar` | Foreach iteration variable | `ty: TypeExpr<'ast>`, `name: Ident<'ast>`, `span: Span` |

**Notes:**
- `vars` is a slice supporting one or more iteration variables. Single-variable foreach has a one-element slice; key-value foreach has two elements.
- Each `ForeachVar` carries its own `TypeExpr` and `Ident`, allowing different types for each iteration variable.
- The `expr` field holds the container expression (the part after the `:`).
- `foreach` is parsed as a contextual keyword -- the parser checks for the identifier `foreach` rather than a dedicated token kind.

## Related Features
- [For Loop](./for-loop.md) - index-based loop alternative
- [While Loop](./while-loop.md) - general condition-based loop
- [Break / Continue](./break-continue.md) - loop control statements
- [Variable Declarations](./variable-declarations.md) - iteration variables follow similar scoping rules
- [Statement Blocks](./statement-blocks.md) - the foreach loop creates an implicit scope
