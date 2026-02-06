# Statement Blocks

## Overview
A statement block is a sequence of statements enclosed in curly braces (`{` and `}`). Each block introduces a new lexical scope: variables declared within the block are only visible inside it and any nested sub-blocks. Statement blocks are used as the body of functions, loops, conditionals, and can also stand alone to create an explicit scope.

## Syntax
```angelscript
// Standalone block
{
    // statements
}

// Nested blocks
{
    int a;
    float b;

    {
        float a;  // shadows outer 'a'
        b = a;    // uses inner 'a' (float), outer 'b'
    }

    // 'a' refers to the int again
}
```

## Semantics
- A statement block is delimited by `{` and `}`.
- Each block creates a new scope. Variables declared within the block are visible only within that block and any blocks nested inside it.
- When execution exits a block, all variables declared in that block go out of scope and are no longer accessible.
- **Variable shadowing:** An inner block can declare a variable with the same name as a variable in an outer block. The inner declaration shadows (hides) the outer one within the inner block's scope. After the inner block ends, the outer variable is visible again with its original value.
- Variables from outer blocks remain visible inside inner blocks, unless shadowed.
- Statement blocks are used implicitly as the bodies of `if`, `else`, `while`, `do-while`, `for`, `switch`, `try`, `catch`, and function definitions.
- An empty block `{}` is legal and has no effect.

## Examples
```angelscript
void example()
{
    int a = 1;
    float b = 2.0f;

    {
        // inner block: new scope
        float a = 3.0f;  // shadows outer 'a' (int)
        b = a;            // b = 3.0f (uses inner 'a', outer 'b')
        int c = 10;       // only visible in this block
    }

    // a is the int again (value 1)
    // b is now 3.0f (modified in inner block)
    // c is not accessible here

    {
        // another block at the same level
        string a = "hello";  // shadows outer 'a' again, different type
    }
}
```

## Compilation Notes
- **Scope management:** The compiler maintains a scope stack. Entering a block pushes a new scope; exiting pops it. Variable lookups traverse the scope stack from innermost to outermost, which implements shadowing naturally.
- **Variable allocation:** Variables declared in a block are allocated in the function's local variable table. The compiler may reuse variable slots from blocks that have ended for later blocks at the same nesting level, since their lifetimes do not overlap.
- **Destructors and cleanup:** When a block exits, the compiler must emit bytecode to:
  - Call destructors for any object-type variables declared in the block (in reverse declaration order).
  - Release (decrement reference counts of) any handle-type variables declared in the block.
  - Primitive variables need no cleanup.
- **Stack behavior:** The variable slots for a block are conceptually "pushed" when the block is entered and "popped" when it exits. The compiler must track the stack depth and ensure it returns to the pre-block depth after cleanup.
- **Shadowing implementation:** When a variable in an inner scope shadows an outer variable, the compiler assigns it a separate slot. The inner slot is used for name resolution within the inner scope. No bytecode is needed for shadowing itself -- it is purely a compile-time name resolution concern.
- **Empty blocks:** An empty block `{}` generates no bytecode (no variables to allocate or clean up). It may still be present in the AST for scope delineation.
- **Special cases:**
  - Function bodies are statement blocks with parameters as pre-declared variables.
  - Loop bodies and conditional bodies are statement blocks. Their cleanup is integrated with the control flow of their parent construct (e.g., loop variables are cleaned up each iteration for body-scoped variables, and on loop exit for init-scoped variables).
  - Early exit from a block (via `return`, `break`, `continue`) must trigger cleanup for all variables in the block before the jump.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/stmt.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Stmt::Block` | Block statement variant | Wraps `Block` (by value) |
| `Block` | Statement block structure | `stmts: &'ast [Stmt<'ast>]`, `span: Span` |

**Notes:**
- `Block` is used both as a standalone statement (`Stmt::Block`) and as a component of other AST nodes (e.g., `TryCatchStmt` embeds `Block` directly for its try and catch bodies).
- An empty block `{}` is represented as a `Block` with an empty `stmts` slice.
- `Block` is stored by value in `Stmt::Block`, not behind a reference, since it is `Copy` (contains a slice reference and a `Span`).

## Related Features
- [Variable Declarations](./variable-declarations.md) - variables are scoped to their enclosing block
- [If-Else](./if-else.md) - if/else bodies are statement blocks
- [While Loop](./while-loop.md) - loop body is a statement block
- [For Loop](./for-loop.md) - the for statement creates an implicit block scope for init variables
- [Return Statement](./return-statement.md) - must clean up all enclosing block scopes
- [Break / Continue](./break-continue.md) - must clean up scopes being exited
