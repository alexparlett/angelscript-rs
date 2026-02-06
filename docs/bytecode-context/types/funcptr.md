# Function Pointers (Funcdefs and Delegates)

## Overview

A function handle is a data type that holds a reference to a function (global function or bound method) matching a specific signature defined by a `funcdef`. Function handles enable callbacks, event systems, and strategy patterns where the exact function to call is not known at compile time. Delegates extend this concept by binding a class method to a specific object instance.

## Syntax

### Funcdef declaration

```angelscript
// Declare a function signature (at global scope or as a class member)
funcdef bool CALLBACK(int, int);
funcdef void EventHandler(const string &in);
funcdef int Transformer(int);
```

### Function handle variables

```angelscript
// Declare a function handle (initialized to null)
CALLBACK@ func;

// Assign a global function
CALLBACK@ func = @myCompare;

// Assign with explicit handle syntax
@func = @myCompare;

// Clear the handle
@func = null;

// Null check
if (func is null) { }
if (func !is null) { }
```

### Calling through a handle

```angelscript
// Call the function through the handle (same syntax as a direct call)
bool result = func(1, 2);
```

### Delegates (bound method handles)

```angelscript
// Bind a class method to an object instance
class MyClass {
    bool compare(int a, int b) { return a > b; }
}

MyClass obj;
CALLBACK@ func = CALLBACK(obj.compare);   // Create delegate

// Call the delegate (calls obj.compare)
func(1, 2);
```

### Anonymous functions (lambdas)

```angelscript
// Inline function definition
funcdef bool CMP(int, int);

void main() {
    CMP@ cmp = function(a, b) { return a < b; };

    // Or pass directly to a function expecting a funcdef handle
    array<int> arr = {3, 1, 2};
    arr.sort(function(a, b) { return a < b; });
}
```

## Semantics

### Funcdef

- A `funcdef` declares a function signature type (return type and parameter types).
- It does not define any implementation.
- Variables of the funcdef type are handles (`@`) that can point to any function with a matching signature.
- Funcdefs can be declared at global scope or as members of a class.

### Function handle assignment

- `@func = @globalFunction`: Assigns the handle to point to the named global function. The function must match the funcdef signature exactly.
- The compiler verifies signature compatibility at compile time.
- Function handles are reference-counted like other handles.

### Calling through handles

- Syntax is identical to a direct function call: `func(args...)`.
- If the handle is null, calling through it raises a script exception.
- The function handle carries all necessary information to invoke the target function.

### Delegates

- A delegate binds a class method to a specific object instance.
- Created with the syntax: `FUNCDEF(objectInstance.methodName)`.
- The delegate holds a reference to both the object and the method.
- The object's reference count is incremented (the delegate keeps the object alive).
- When the delegate is called, it invokes the method on the bound object, with the provided arguments.
- The bound method must match the funcdef signature (excluding the implicit `this` parameter).

### Anonymous functions (lambdas)

- Defined inline using `function(params) { body }`.
- The signature (parameter types and return type) is inferred from the target funcdef.
- Parameter and return types can be explicitly specified for disambiguation.
- **Lambdas cannot capture variables** from the enclosing scope. They are not closures.
- If the target funcdef is ambiguous (multiple overloads), explicit parameter types are required.

```angelscript
funcdef void A(int);
funcdef void B(float);
void doSomething(A@) {}
void doSomething(B@) {}

void main() {
    // Explicit type needed for disambiguation
    doSomething(function(int a) { });
}
```

### Identity comparison

Function handles support `is` and `!is` for identity comparison:

```angelscript
CALLBACK@ a = @myFunc;
CALLBACK@ b = @myFunc;
if (a is b) { }     // True: both point to the same function
if (a is null) { }   // Check for null
```

## Examples

```angelscript
// Basic callback pattern
funcdef bool CALLBACK(int, int);

bool greaterThan(int a, int b) { return a > b; }
bool lessThan(int a, int b) { return a < b; }

void sortWithCallback(array<int>@ arr, CALLBACK@ cmp) {
    // Use cmp to compare elements
    arr.sort(cmp);
}

void main() {
    array<int> numbers = {5, 3, 8, 1, 4};

    sortWithCallback(numbers, @greaterThan);  // Sort descending
    sortWithCallback(numbers, @lessThan);     // Sort ascending
}

// Delegate pattern
funcdef void OnClick();

class Button {
    OnClick@ callback;

    void click() {
        if (callback !is null) {
            callback();
        }
    }
}

class App {
    int clickCount = 0;

    void handleClick() {
        clickCount++;
        print("Clicked " + clickCount + " times\n");
    }
}

void main() {
    App app;
    Button btn;

    // Create delegate binding app.handleClick
    @btn.callback = OnClick(app.handleClick);

    btn.click();   // "Clicked 1 times"
    btn.click();   // "Clicked 2 times"
}

// Lambda / anonymous function
funcdef int Transform(int);

int applyTransform(int value, Transform@ t) {
    return t(value);
}

void main() {
    int result = applyTransform(5, function(x) { return x * x; });
    print("Result: " + result + "\n");   // "Result: 25"
}
```

## Compilation Notes

- **Memory layout:** A function handle is stored as a pointer-sized value on the stack. For global function handles, it points to the function's entry point (or a function descriptor). For delegates, it points to a delegate object that contains both the function pointer and the object instance pointer. Delegates are reference-counted objects.
- **Stack behavior:** Function handles occupy one pointer-sized slot. Handle assignment involves addref/release. Delegate creation allocates a delegate object on the heap with refcount = 1.
- **Type considerations:**
  - The compiler must verify at compile time that the assigned function's signature matches the funcdef exactly (parameter types, return type, constness).
  - For delegates, the compiler must verify that the method's signature (excluding `this`) matches the funcdef.
  - Calling through a function handle emits an indirect call instruction. The bytecode must load the function pointer from the handle, push arguments, and emit a `CALLPTR` or equivalent indirect call instruction.
  - For delegates, the call must additionally push the bound object as the `this` pointer before the method call.
  - Lambdas are compiled as anonymous global functions. The compiler generates a unique function with the inferred signature and emits a handle to it.
- **Lifecycle:**
  - Function handle creation (global function): Store the function pointer, addref the handle.
  - Delegate creation: Allocate a delegate object, store the function pointer and the object reference (addref the object), set delegate refcount = 1.
  - Function handle release: Decref the handle. For delegates, when the delegate refcount reaches zero, release the bound object and free the delegate.
  - Lambda: The anonymous function is compiled into the module's function table. Creating a handle to it is the same as creating a handle to any global function.
- **Special cases:**
  - Null checks must be emitted before indirect calls to raise a proper exception.
  - The funcdef type itself is registered in the type system and has a unique type ID. This allows function handles to be stored in containers (e.g., `array<CALLBACK@>`).
  - When a funcdef is a class member, the member funcdef creates a nested funcdef type scoped to the class.
  - Delegate objects must be distinguishable from plain function handles at runtime (different call mechanics).

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/types.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `TypeBase::Named(Ident { name: "<funcdef_name>", .. })` | Funcdef type used as a type name | Wraps `Ident` with the funcdef's declared name (e.g., `"CALLBACK"`) |
| `TypeSuffix::Handle` | Handle suffix on funcdef type (`CALLBACK@`) | `is_const: bool` |

**Notes:**
- Funcdef names (e.g., `CALLBACK`, `EventHandler`) appear in the type AST as `TypeBase::Named(Ident { name: "CALLBACK", .. })`. The parser treats them identically to any other user-defined type name.
- Function handle variables like `CALLBACK@ func` are represented as `TypeExpr { base: TypeBase::Named("CALLBACK"), suffixes: &[TypeSuffix::Handle { is_const: false }], .. }`.
- The `funcdef` declaration itself is a top-level statement, not a type expression. The type AST only appears when a funcdef name is used as a type (e.g., in variable declarations or parameter types).
- Delegate creation (`CALLBACK(obj.method)`) and lambda expressions (`function(a, b) { ... }`) are expression-level constructs, not part of the type AST.
- The parser does not validate that a named type is a funcdef versus a class or other type; that is deferred to semantic analysis.
- See also: [function-declarations.md](../functions/function-declarations.md) for `ReturnType` and `ParamType` usage in funcdef signatures.

## Related Features

- [Object handles (handle semantics)](./handles.md)
- [Objects (delegate binding to class instances)](./objects.md)
- [Arrays (sort callbacks)](./arrays.md)
- [Auto declarations (type inference with function handles)](./auto-declarations.md)
