# Coroutines

## Overview

Co-routines in AngelScript allow multiple execution paths to run cooperatively in parallel without the hazards of pre-emptive multithreading. Unlike threads, co-routines always voluntarily suspend themselves in favor of the next co-routine, eliminating the need for atomic instructions and critical sections.

Co-routines are **not** built into the AngelScript language or VM as a native feature. They are implemented entirely at the application/host level using the script context API. The Context Manager add-on provides a default implementation.

## Syntax

There is no dedicated AngelScript syntax for coroutines. From the script side, coroutines are used through application-registered functions:

```angelscript
// Spawning a new co-routine (application-provided function)
void createCoRoutine(const string &in functionName);

// Yielding control to the next co-routine (application-provided function)
void yield();
```

Script-side usage:

```angelscript
void main()
{
    createCoRoutine("workerA");
    createCoRoutine("workerB");
}

void workerA()
{
    for (int i = 0; i < 5; i++)
    {
        // Do some work
        yield();  // Give other co-routines a chance to run
    }
}

void workerB()
{
    for (int i = 0; i < 3; i++)
    {
        // Do some work
        yield();
    }
}
```

## Semantics

### Core Components

A coroutine system requires the following pieces:

1. **Context per coroutine** -- Each co-routine is an `asIScriptContext` instance that holds its own independent call stack.
2. **Spawn function** -- Creates a new context, prepares it with a starting function, and appends it to the co-routine array. The starting function can be identified by name or by function pointer.
3. **Yield function** -- Calls `ctx->Suspend()` on the currently active context, causing the VM to return `asEXECUTION_SUSPENDED` from `Execute()`.
4. **Scheduler/control loop** -- Iterates over the array of co-routine contexts. When a context returns `asEXECUTION_SUSPENDED`, it moves to the next. When a context finishes (returns anything other than suspended), it is released and removed.

### Execution Model

- Co-routines are **cooperatively** scheduled. A co-routine runs until it either completes or explicitly yields.
- Yielding is implemented by calling `Suspend()` on the active context, which causes the current `Execute()` call to return `asEXECUTION_SUSPENDED`.
- The suspended context retains its full call stack and local variables. Calling `Execute()` again on a suspended context resumes from where it left off.
- New co-routines spawned during execution are appended to the co-routine array and will be picked up when the current co-routine yields.

### Context States

| Return Code | Meaning |
|-------------|---------|
| `asEXECUTION_SUSPENDED` | Co-routine yielded; resume later with `Execute()` |
| `asEXECUTION_FINISHED` | Co-routine completed normally |
| `asEXECUTION_EXCEPTION` | Co-routine terminated with an unhandled exception |

### Lifetime Management

- Each co-routine context is reference counted. When a co-routine finishes, its context should be released (`Release()`).
- The scheduler is responsible for managing the lifetime of all co-routine contexts.

## Examples

### Spawning a Co-routine by Name

```angelscript
void createCoRoutine(string &func)
{
    // Get the active context to find the engine/module
    asIScriptContext *ctx = asGetActiveContext();
    asIScriptEngine *engine = ctx->GetEngine();
    string mod = ctx->GetFunction()->GetModuleName();

    // Find the target function
    string decl = "void " + func + "()";
    asIScriptFunction *funcPtr = engine->GetModule(mod)->GetFunctionByDecl(decl);
    if (funcPtr == 0)
    {
        ctx->SetException("Function not found");
        return;
    }

    // Create and prepare a new context
    asIScriptContext *coctx = engine->CreateContext();
    coctx->Prepare(funcPtr);
    coroutines.push_back(coctx);
}
```

### Yield Function

```cpp
void Yield()
{
    asIScriptContext *ctx = asGetActiveContext();
    if (ctx)
    {
        ctx->Suspend();
    }
}
```

### Round-Robin Scheduler

```cpp
std::vector<asIScriptContext *> coroutines;

void Execute()
{
    int n = 0;
    while (coroutines.size() > 0)
    {
        int r = coroutines[n]->Execute();
        if (r == asEXECUTION_SUSPENDED)
        {
            // Move to next co-routine
            if (++n >= coroutines.size())
                n = 0;
        }
        else
        {
            // Co-routine finished, release it
            coroutines[n]->Release();
            coroutines.erase(coroutines.begin() + n);
            if (n >= coroutines.size())
                n = 0;
        }
    }
}
```

## Compilation Notes

- **Runtime support:** Coroutines are entirely a runtime/host concept. The compiler does not generate any special bytecode for coroutines. The `yield()` and `createCoRoutine()` functions are ordinary registered application functions.
- **Stack behavior:** Each coroutine maintains its own `asIScriptContext`, which holds the complete call stack and all local variables. Suspending preserves the full execution state. Resuming restores execution at the exact instruction where suspension occurred.
- **Bytecode interaction:** The `asBC_SUSPEND` bytecode instruction is used by the VM to save state and return control to the application. When `Suspend()` is called from within an application-registered function, the VM sets a flag so that the next opportunity to check suspension (at a suspend point in the bytecode) will cause `Execute()` to return `asEXECUTION_SUSPENDED`.
- **JIT considerations:** JIT-compiled functions must handle the `asBC_SUSPEND` instruction by returning control to the VM, which then manages the suspension. The JIT function cannot independently suspend execution.
- **No language-level cost:** Since coroutines are not a language feature, there is zero overhead in compiled scripts that do not use coroutine-related registered functions.
- **Special cases:** If a co-routine raises an unhandled exception, the scheduler receives `asEXECUTION_EXCEPTION` and should clean up that context. The scheduler implementation determines whether one co-routine's exception affects others.

## AST Mapping

No direct AST representation -- coroutines are a runtime/VM feature implemented entirely at the host application level. The `yield()` and `createCoRoutine()` functions are ordinary registered application functions with no special syntax or AST nodes. Script functions used as coroutine entry points are standard `FunctionDecl` nodes.

## Related Features

- [try-catch](../statements/try-catch.md) -- Exception handling within a coroutine
- [function-calls](../expressions/function-calls.md) -- Calling registered application functions
- [function-declarations](../functions/function-declarations.md) -- Declaring functions used as coroutine entry points
