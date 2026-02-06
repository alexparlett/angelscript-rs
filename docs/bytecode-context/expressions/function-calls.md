# Function Calls

## Overview
Function calls invoke a named function or method, optionally passing arguments and receiving a return value. AngelScript supports positional arguments, named arguments, output reference parameters, and a special `void` argument to discard output values.

## Syntax
```angelscript
// No arguments
func();

// Positional arguments
func(arg);
func(arg1, arg2);

// Capturing return value
lvalue = func();

// Output parameters
func(outputVar);
func(void);          // discard output value

// Named arguments
func(paramName: value);
func(flagB: true, flagA: true);
```

## Semantics
- If a function takes more than one argument, argument expressions are evaluated in **reverse order** (last argument evaluated first).
- Functions may be declared with output reference parameters (`&out`). The caller must provide an lvalue to receive the output, or pass the keyword `void` to discard the output.
- Named arguments allow specifying a parameter by name using the syntax `paramName: value`. Once a named argument is used, no positional arguments may follow.
- Named arguments can appear in any order relative to the parameter declaration order.
- All parameters with default values may be omitted if not needed.
- Method calls on objects use the member access operator: `object.method(args)`.
- The return value of a function can be used as an rvalue in any expression context. If the function returns `void`, its result cannot be used in an expression.

## Examples
```angelscript
// Basic function call
int result = add(3, 5);

// Output parameter
void getValues(int &out x, int &out y) {
    x = 10;
    y = 20;
}

int a, b;
getValues(a, b);     // a = 10, b = 20
getValues(void, b);  // discard first output, b = 20

// Named arguments
void configure(int width = 800, int height = 600, bool fullscreen = false) {}

configure(fullscreen: true);                    // only set fullscreen
configure(height: 1080, width: 1920);           // set both, any order

// Method call
obj.doSomething(42);
```

## Compilation Notes
- **Evaluation order:** Arguments are evaluated in reverse order (rightmost first). This means `arg2` is evaluated and pushed before `arg1` for a call like `func(arg1, arg2)`. The function then finds arguments in the expected order on the stack.
- **Stack behavior:**
  - Each argument is evaluated and pushed onto the stack in reverse order.
  - For `&out` parameters, the address of the lvalue is pushed; for `void` arguments, a null/discard marker is pushed.
  - The call instruction transfers control. On return, arguments are popped and the return value (if any) is pushed.
- **Type considerations:**
  - Argument types are checked against parameter types at compile time. Implicit conversions are inserted as needed.
  - For overloaded functions, the compiler performs overload resolution based on conversion cost.
  - Named arguments require the compiler to map argument positions to parameter indices before emitting push instructions.
- **Special cases:**
  - `void` arguments for output parameters: the compiler must detect this keyword and either skip the output write-back or provide a temporary discard target.
  - Named arguments may require reordering the evaluation sequence to match the declared parameter order after remapping.
  - Default parameter values: the compiler inserts the default value expression for any omitted parameters.
  - Method calls require pushing the object reference (this pointer) in addition to the arguments.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/expr.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Expr::Call` | Function call expression variant | Wraps `&CallExpr` |
| `CallExpr` | Function call | `callee: &Expr`, `args: &[Argument]`, `span: Span` |
| `Argument` | Function call argument | `name: Option<Ident>`, `value: &Expr`, `span: Span` |

**Notes:**
- `CallExpr.callee` is any `Expr`, allowing calls on identifiers (`foo()`), scoped identifiers (`NS::foo()`), and expressions producing callables.
- Named arguments are represented by `Argument.name: Some(Ident)`. Positional arguments have `name: None`.
- Method calls (`obj.method(args)`) are **not** represented as `Expr::Call`. Instead they use `Expr::Member` / `MemberAccess::Method`, which bundles the method name and arguments together. See [member-access.md](member-access.md).
- `Expr::Call` is also used for constructor-style casts and anonymous object construction (`TypeName(args)`), where the callee is an `Expr::Ident` with the type name. Disambiguation happens during semantic analysis.
- `Expr::Lambda` / `LambdaExpr` represents anonymous function definitions (primary doc: `functions/anonymous-functions.md`, not in scope for this file).

## Related Features
- [member-access.md](member-access.md) - Method calls through dot operator
- [type-conversions.md](type-conversions.md) - Argument type coercion
- [anonymous-objects.md](anonymous-objects.md) - Passing anonymous objects as arguments
