# Function References

## Overview
Function references in AngelScript allow functions to be treated as first-class values through **funcdefs** (function type definitions) and **function handles**. A funcdef declares a function signature type, and a handle (`@`) to that type can hold a reference to any function (or anonymous function) matching the signature. This enables callback patterns, strategy patterns, and event-driven architectures.

## Syntax

### Declaring a funcdef
```angelscript
funcdef bool CMP(int, int);
funcdef void Callback(string);
funcdef int Transform(int);
```

A funcdef declares a named function type with a specific return type and parameter list.

### Creating a function handle
```angelscript
funcdef void MyFunc(int);

void PrintInt(int val) {
    print("" + val);
}

void main() {
    MyFunc @f = PrintInt;   // Handle to named function
    f(42);                  // Call through handle
}
```

### Assigning anonymous functions
```angelscript
funcdef int Op(int, int);

void main() {
    Op @add = function(a, b) { return a + b; };
    Op @mul = function(a, b) { return a * b; };

    print("" + add(3, 4));   // 7
    print("" + mul(3, 4));   // 12
}
```

### Passing function handles as parameters
```angelscript
funcdef bool Predicate(int);

int CountMatching(int[] &arr, Predicate @pred) {
    int count = 0;
    for (uint i = 0; i < arr.length(); i++) {
        if (pred(arr[i]))
            count++;
    }
    return count;
}

bool IsPositive(int val) { return val > 0; }

void main() {
    int[] data = {-1, 2, -3, 4, 5};
    int pos = CountMatching(data, IsPositive);  // 3
}
```

### Delegate creation from class methods
```angelscript
funcdef void Handler(int);

class MyClass {
    void OnEvent(int code) {
        // Handle event
    }
}

void main() {
    MyClass obj;
    Handler @h = Handler(obj.OnEvent);  // Delegate: binds method to object
    h(42);   // Calls obj.OnEvent(42)
}
```

When creating a delegate from a method, the resulting handle binds both the object instance and the method together. The object's reference count is incremented to keep it alive as long as the delegate exists.

## Semantics

### Funcdef matching rules
A function handle can reference any function (named or anonymous) whose signature matches the funcdef:
- Same number of parameters
- Each parameter type matches exactly (or is implicitly convertible following the standard conversion rules)
- Return type matches exactly

### Null handles
A function handle can be null:
```angelscript
funcdef void Callback();

Callback @cb = null;
if (cb !is null)
    cb();
```

Calling a null function handle results in a runtime exception.

### Handle comparison
Function handles can be compared for identity:
```angelscript
funcdef void F();

void A() {}
void B() {}

void main() {
    F @f1 = A;
    F @f2 = A;
    F @f3 = B;

    bool same = (f1 is f2);     // true: both point to A
    bool diff = (f1 is f3);     // false: different functions
}
```

### Delegates (bound methods)
A delegate binds a specific object instance to a method. The delegate holds:
- A reference to the object (preventing it from being garbage collected)
- A reference to the method

When called, the delegate invokes the method on the bound object, effectively injecting `this`.

### Storing and passing handles
Function handles are reference-counted objects. They can be:
- Stored in variables
- Passed as function arguments
- Returned from functions
- Stored in arrays or class members

```angelscript
funcdef int Op(int, int);

class Calculator {
    Op @operation;

    void SetOp(Op @op) {
        @operation = op;
    }

    int Execute(int a, int b) {
        return operation(a, b);
    }
}
```

## Examples

### Event system
```angelscript
funcdef void EventHandler(string eventName);

class EventEmitter {
    array<EventHandler@> handlers;

    void On(EventHandler @h) {
        handlers.insertLast(h);
    }

    void Emit(string name) {
        for (uint i = 0; i < handlers.length(); i++)
            handlers[i](name);
    }
}

void LogEvent(string name) {
    print("Event: " + name);
}

void main() {
    EventEmitter emitter;
    emitter.On(LogEvent);
    emitter.On(function(name) { print("Also got: " + name); });
    emitter.Emit("click");
}
```

### Strategy pattern
```angelscript
funcdef float PricingStrategy(float basePrice, int quantity);

float BulkDiscount(float price, int qty) {
    return qty >= 10 ? price * 0.9f : price;
}

float NoDiscount(float price, int qty) {
    return price;
}

float CalculateTotal(float unitPrice, int qty, PricingStrategy @strategy) {
    return strategy(unitPrice, qty) * qty;
}
```

## Compilation Notes
- **Funcdef as a type:** A funcdef introduces a new type into the type system. The compiler treats `funcdef` handles like any other object handle, with reference counting and null checks.
- **Direct vs. indirect calls:** When a function is called by name, the compiler emits a direct `CALL` instruction with the function's address. When called through a handle, the compiler emits an indirect call instruction that loads the function address from the handle object at runtime.
- **Delegate object layout:** A delegate stores two pointers: one to the bound object and one to the function. The object pointer is reference-counted. The delegate itself is a small heap-allocated object with its own reference count.
- **Handle assignment:** Assigning a named function to a funcdef handle generates bytecode that creates a handle object wrapping the function's address. For anonymous functions, the generated hidden function's address is used.
- **Null handle call:** The runtime checks the handle for null before performing an indirect call. A null dereference triggers a script exception.
- **Stack frame for indirect calls:** The calling convention for indirect calls through handles is identical to direct calls. The caller pushes arguments, the indirect call instruction resolves the target, and the callee executes with a standard stack frame. For delegates, the `this` pointer is injected as an implicit first parameter by the delegate dispatch mechanism.
- **Reference counting overhead:** Each assignment to or from a function handle involves incrementing and decrementing reference counts. For delegates, this also applies to the bound object reference.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/decl.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `FuncdefDecl` | Function type definition (`funcdef`) | `modifiers: DeclModifiers`, `return_type: ReturnType`, `name: Ident`, `template_params: &[Ident]`, `params: &[FunctionParam]`, `span: Span` |
| `FunctionParam` | Funcdef parameter | `ty: ParamType`, `name: Option<Ident>`, `default: Option<&Expr>`, `is_variadic: bool`, `span: Span` |

**Notes:**
- Function references rely on `FuncdefDecl` for declaring the function signature type. The funcdef itself is a top-level declaration (`Item::Funcdef`), or can appear nested inside a class (`ClassMember::Funcdef`).
- Handle creation, delegate binding, null checks, and indirect calls are all **semantic** operations resolved during compilation. They have no dedicated AST nodes; they are expressed through existing expression nodes (assignments, function calls, `is` comparisons).
- `FuncdefDecl.template_params` is for application-registered template funcdefs and is not part of standard script-level syntax.

## Related Features
- [Function Declarations](./function-declarations.md)
- [Anonymous Functions](./anonymous-functions.md)
- [Function Overloading](./function-overloading.md)
