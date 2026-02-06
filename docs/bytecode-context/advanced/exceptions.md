# Exception Handling

## Overview

AngelScript supports exception handling through `try`/`catch` blocks in script code and through the `throw()` and `getExceptionInfo()` standard library functions. Exceptions can be raised by the VM itself (e.g., null pointer access, division by zero), by the host application via the C++ API, or explicitly by scripts using the `throw()` function.

The standard `throw` and `getExceptionInfo` functions are **only available** if the application registers them (via the exception helper add-on). They are not built-in language keywords.

## Syntax

### try-catch Block

```angelscript
try
{
    DoSomethingThatMightThrow();
    // Statements after the throwing call are NOT executed
}
catch
{
    // Executed if any exception was thrown in the try block
    string info = getExceptionInfo();
}
```

### throw Function

```angelscript
void throw(const string &in exception)
```

Explicitly throws an exception with a descriptive string. The string should identify the type or cause of the exception for logging or handling purposes.

```angelscript
throw("Invalid argument: value must be positive");
```

### getExceptionInfo Function

```angelscript
string getExceptionInfo()
```

Returns the exception string for the most recently thrown exception. Typically called inside a `catch` block.

```angelscript
try
{
    riskyOperation();
}
catch
{
    string error = getExceptionInfo();
    print("Caught exception: " + error);
}
```

## Semantics

### Exception Sources

Exceptions can originate from several sources:

| Source | Example | Exception String |
|--------|---------|-----------------|
| Null pointer dereference | Accessing member on null handle | "Null pointer access" |
| Division by zero | `int x = 10 / 0;` | "Divide by zero" |
| Out of bounds | Array index out of range | "Index out of bounds" |
| Application-raised | Host calls `ctx->SetException(...)` | Application-defined string |
| Script-raised | `throw("message")` | The string passed to throw |
| Overflow | Integer overflow on checked operations | "Overflow" |

### try-catch Behavior

- The `try` block is entered and statements execute sequentially.
- If an exception occurs at any point within the `try` block, execution immediately transfers to the `catch` block. Remaining statements in the `try` block are skipped.
- The `catch` block has no parameter -- AngelScript does not have typed exception objects. The exception is always a string retrievable via `getExceptionInfo()`.
- If no exception occurs, the `catch` block is skipped entirely.
- After the `try`/`catch` construct, execution continues normally with the next statement.

### Exception Propagation

- If an exception is thrown and there is **no** enclosing `try`/`catch`, the exception propagates up the call stack.
- If the exception reaches the top of the call stack without being caught, `Execute()` returns `asEXECUTION_EXCEPTION` to the host application.
- The host can then inspect the exception using `ctx->GetExceptionString()`, `ctx->GetExceptionFunction()`, and `ctx->GetExceptionLineNbr()`.

### Nested try-catch

```angelscript
try
{
    try
    {
        throw("inner error");
    }
    catch
    {
        // Catches the inner error
        string inner = getExceptionInfo();  // "inner error"
        throw("outer error");  // Re-throw or throw new
    }
}
catch
{
    string outer = getExceptionInfo();  // "outer error"
}
```

### SetException from Host

The host application can set an exception on the active context from within a registered function:

```cpp
void MyRegisteredFunction()
{
    asIScriptContext *ctx = asGetActiveContext();
    ctx->SetException("Something went wrong", true);  // true = allow catch
}
```

The second parameter (`allowCatch`) controls whether the exception can be caught by a script-level `try`/`catch`:
- `true` -- Exception can be caught in script
- `false` -- Exception always propagates to the host (bypasses script catch blocks)

### Object Cleanup During Stack Unwinding

When an exception propagates through the call stack:
- Local variables with destructors are properly destroyed.
- Reference-counted handles are released.
- Value types on the stack are destructed.
- The stack is unwound frame by frame until a `catch` block is found or the top of the stack is reached.

## Examples

### Basic Exception Handling

```angelscript
void safeDivide(int a, int b)
{
    try
    {
        int result = a / b;
        print("Result: " + result);
    }
    catch
    {
        print("Error: " + getExceptionInfo());
    }
}

safeDivide(10, 0);  // Prints: Error: Divide by zero
```

### Guard Pattern

```angelscript
bool tryLoadResource(const string &in path)
{
    try
    {
        Resource@ res = LoadResource(path);
        res.Initialize();
        return true;
    }
    catch
    {
        LogError("Failed to load " + path + ": " + getExceptionInfo());
        return false;
    }
}
```

### Explicit Throw

```angelscript
void validateAge(int age)
{
    if (age < 0)
        throw("Age cannot be negative: " + age);
    if (age > 150)
        throw("Age is unreasonably large: " + age);
}

try
{
    validateAge(-5);
}
catch
{
    print(getExceptionInfo());  // "Age cannot be negative: -5"
}
```

## Compilation Notes

- **Runtime support:** Exception handling requires the VM to maintain a stack of exception handler records (try/catch boundaries). When entering a `try` block, the VM pushes a handler record. When leaving (normally or via exception), it pops the record.
- **Stack behavior:** On exception, the VM performs stack unwinding: it walks back through stack frames, calling destructors for local objects and releasing reference-counted handles, until it finds a matching `catch` handler or exhausts the call stack.
- **Bytecode interaction:** The compiler generates bytecode to:
  - Mark the beginning of a `try` block (push exception handler)
  - Mark the transition to the `catch` block
  - Mark the end of the `try`/`catch` construct (pop exception handler)
  - The `throw()` function is a regular registered function call (`asBC_CALLSYS`) that internally calls `SetException()` on the active context
- **Type considerations:** AngelScript exceptions are untyped strings, not objects. There is no exception type hierarchy or typed catch blocks. The `getExceptionInfo()` function returns the string regardless of the exception source.
- **Special cases:**
  - Division instructions (`asBC_DIVi`, `asBC_DIVf`, etc.) check for zero divisor and raise an exception internally rather than executing undefined behavior.
  - Null pointer dereference checks (`asBC_CHKREF`, `asBC_ChkRefS`, `asBC_ChkNullV`, `asBC_ChkNullS`) raise exceptions when the pointer is null.
  - The `asBC_SUSPEND` instruction provides a safe point for the VM to check for pending exceptions set by the host.
- **Performance:** Code outside of `try`/`catch` blocks has no exception-handling overhead. The cost is paid only when entering a `try` block (pushing a handler) and when an exception actually occurs (stack unwinding).

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/stmt.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Stmt::TryCatch` | Statement variant for try-catch blocks | Wraps `&TryCatchStmt` |
| `TryCatchStmt` | Try-catch statement | `try_block: Block<'ast>`, `catch_block: Block<'ast>`, `span: Span` |
| `Block` | Block of statements (used for both try and catch bodies) | `stmts: &[Stmt<'ast>]`, `span: Span` |

**Notes:**
- The `throw()` and `getExceptionInfo()` functions have no special AST representation -- they are ordinary registered application function calls, represented as `Expr::FunctionCall` nodes.
- `TryCatchStmt` has no exception variable or type parameter in the `catch` block, consistent with AngelScript's untyped string exceptions. The exception info is retrieved via the `getExceptionInfo()` function call.
- The AST does not distinguish between VM-raised exceptions (null pointer, division by zero) and script-raised exceptions (`throw()`). Both follow the same try-catch control flow.

## Related Features

- [try-catch](../statements/try-catch.md) -- Statement-level try/catch syntax
- [function-calls](../expressions/function-calls.md) -- Calling throw() and getExceptionInfo()
- [handles](../types/handles.md) -- Null handle dereference as exception source
- [math-operators](../expressions/math-operators.md) -- Division by zero as exception source
- [coroutines](coroutines.md) -- Exception behavior within co-routines
