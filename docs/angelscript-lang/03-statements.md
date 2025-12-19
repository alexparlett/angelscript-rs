# Statements

## Variable Declarations

```angelscript
int var = 0, var2 = 10;      // Multiple variables, same type
object@ handle, handle2;      // Handle declarations
const float pi = 3.141592f;   // Constant declaration
```

**Rules:**
- Variables must be declared before use within the statement block
- Variables are scoped to the block they're declared in
- Multiple variables can be declared on one line (comma-separated, same type)
- `const` variables cannot be modified after initialization
- **Primitives without initializers have undefined values**
- **Handles without initializers are `null`**
- **Objects without initializers use their default constructor**

## Expression Statement

```angelscript
a = b;    // assignment
func();   // function call
```

All expression statements must end with `;`.

## Conditions

### if / if-else

```angelscript
if (condition) {
    // executed if condition is true
}

if (value < 10) {
    // value less than 10
} else {
    // value >= 10
}
```

The condition must evaluate to `bool` (`true` or `false`).

### switch-case

```angelscript
switch (value) {
case 0:
    // value == 0
    break;

case 2:
case constant_value:   // can use const variables
    // value == 2 or value == constant_value
    break;

default:
    // no match
}
```

**Rules:**
- Switch works with integer expressions (signed or unsigned)
- Much faster than chained if-else when cases are close in value
- Each case should end with `break` (fall-through is allowed)
- Case values must be compile-time constants (literal or const variable with constant initializer)

## Loops

### while

```angelscript
int i = 0;
while (i < 10) {
    // condition checked BEFORE body
    i++;
}
```

### do-while

```angelscript
int j = 0;
do {
    // body executed BEFORE condition check
    j++;
} while (j < 10);
```

### for

```angelscript
for (int n = 0; n < 10; n++) {
    // compact loop form
}

// Multiple variables and increment expressions
for (int a = 0, b = 10; a < b; a++, b--) {
    // ...
}
```

**For loop structure:**
1. **Init** (before first `;`): Executed once before loop starts. Can declare variables scoped to loop.
2. **Condition** (between `;`s): Checked before each iteration. Empty = always true.
3. **Increment** (after second `;`): Executed after each iteration.

Multiple expressions in init/increment are separated by `,`.

## Loop Control

### break

```angelscript
for (;;) {  // infinite loop
    if (condition)
        break;  // exits the loop
}
```

Terminates the **innermost enclosing loop or switch**.

### continue

```angelscript
for (int n = 0; n < 10; n++) {
    if (n == 5)
        continue;  // skip to next iteration
    // executed for all n except 5
}
```

Jumps to the next iteration of the **innermost enclosing loop**.

## Return Statement

```angelscript
float valueOfPI() {
    return 3.141592f;
}

void doSomething() {
    if (done)
        return;  // early exit, no value
    // ...
}
```

- Functions with non-void return type **must** end with `return <expression>`
- `void` functions can use `return;` for early exit

## Statement Blocks

```angelscript
{
    int a;
    float b;

    {
        float a;  // shadows outer 'a'
        b = a;    // uses inner 'a', outer 'b'
    }

    // 'a' refers to int again
}
```

Each block creates a new scope. Inner declarations can shadow outer ones.

## Try-Catch

```angelscript
try {
    DoSomethingThatMightThrow();
    // not executed if exception thrown
}
catch {
    // executed if exception was thrown
}
```

**Exception sources:**
- Null pointer access (uninitialized handles)
- Division by zero
- Application-raised exceptions
- Script-raised exceptions (via standard library)

See also: Exception handling in standard library
