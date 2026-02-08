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

### Container Requirements
The expression after the colon (`:`) must evaluate to a type that supports the foreach iterator protocol. This protocol requires the following operator overloads:

| Method | Return Type | Purpose |
|--------|-------------|---------|
| `opForBegin()` | Iterator type (typically `int`, but can be custom) | Initialize iteration, returns starting iterator value |
| `opForEnd(iterator)` | `bool` | Check termination; returns `true` when iteration is complete |
| `opForNext(iterator)` | Iterator type (same as `opForBegin` return) | Advance iterator; returns next iterator value |
| `opForValue(iterator)` | Element type | Retrieve current element (single iteration variable) |
| `opForValue0(iterator)` | First element type | Retrieve first component (multiple iteration variables) |
| `opForValue1(iterator)` | Second element type | Retrieve second component (multiple iteration variables) |
| `opForValue2(iterator)`, etc. | Nth element type | Retrieve additional components (3+ iteration variables) |

Built-in types like arrays and dictionaries already implement this protocol. Custom classes can implement these methods to become iterable.

### Iteration Variables
- Each iteration variable is declared with an explicit type and name. The type must be compatible with what the container yields via `opForValue` (single variable) or `opForValue0`, `opForValue1`, etc. (multiple variables).
- **Single variable:** The variable receives each element value from `opForValue(iterator)`.
- **Multiple variables:** For containers like dictionaries, `opForValue0(iterator)` provides the first component (e.g., key) and `opForValue1(iterator)` provides the second component (e.g., value). More variables use `opForValue2`, `opForValue3`, etc.
- Variables declared in the foreach header are scoped to the foreach statement (header and body).

### Control Flow
- The body can be a single statement or a statement block (curly braces).
- `break` and `continue` can be used within the body. `break` exits the loop entirely; `continue` skips to the next iteration.
- The iteration order depends on the container type (sequential for arrays, implementation-defined for dictionaries).

### Loop Transformation
The foreach loop is syntactic sugar for a for loop using the iterator protocol:

```angelscript
// Single iteration variable:
foreach (Type var : container)
    body;

// Desugars to:
for (auto @iterator = container.opForBegin();
     !container.opForEnd(iterator);
     iterator = container.opForNext(iterator))
{
    Type var = container.opForValue(iterator);
    body;
}
```

```angelscript
// Multiple iteration variables:
foreach (Type1 var1, Type2 var2 : container)
    body;

// Desugars to:
for (auto @iterator = container.opForBegin();
     !container.opForEnd(iterator);
     iterator = container.opForNext(iterator))
{
    Type1 var1 = container.opForValue0(iterator);
    Type2 var2 = container.opForValue1(iterator);
    body;
}
```

## Examples

### Using Built-in Types
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

### Implementing Foreach Protocol in Custom Classes
```angelscript
// Custom container implementing foreach protocol
class IntList
{
    private int[] data;

    void add(int value)
    {
        data.insertLast(value);
    }

    // Foreach iterator protocol - single iteration variable
    int opForBegin() const
    {
        return 0;  // Start at index 0
    }

    bool opForEnd(int iterator) const
    {
        return iterator >= data.length();  // Stop when we reach the end
    }

    int opForNext(int iterator) const
    {
        return iterator + 1;  // Advance to next index
    }

    int opForValue(int iterator) const
    {
        return data[iterator];  // Return element at current index
    }
}

void example()
{
    IntList list;
    list.add(10);
    list.add(20);
    list.add(30);

    // The compiler transforms this into calls to opForBegin/End/Next/Value
    foreach (int val : list)
    {
        print("Value: " + val);
    }
    // Output:
    // Value: 10
    // Value: 20
    // Value: 30
}
```

```angelscript
// Dictionary-like container with multiple iteration variables
class StringMap
{
    private string[] keys;
    private int[] values;

    void set(const string &in key, int val)
    {
        keys.insertLast(key);
        values.insertLast(val);
    }

    // Foreach iterator protocol - multiple iteration variables
    int opForBegin() const
    {
        return 0;
    }

    bool opForEnd(int iterator) const
    {
        return iterator >= keys.length();
    }

    int opForNext(int iterator) const
    {
        return iterator + 1;
    }

    // First iteration variable - key
    string opForValue0(int iterator) const
    {
        return keys[iterator];
    }

    // Second iteration variable - value
    int opForValue1(int iterator) const
    {
        return values[iterator];
    }
}

void example()
{
    StringMap map;
    map.set("health", 100);
    map.set("mana", 50);

    // The compiler transforms this into calls to opForValue0 and opForValue1
    foreach (string key, int value : map)
    {
        print(key + " = " + value);
    }
    // Output:
    // health = 100
    // mana = 50
}
```

### Custom Iterator Type
```angelscript
// Container using a custom iterator type (not just int)
class TreeNode
{
    int value;
    TreeNode@ left;
    TreeNode@ right;
}

class TreeIterator
{
    TreeNode@[] stack;

    TreeIterator(TreeNode@ root)
    {
        if (root !is null)
            pushLeft(root);
    }

    void pushLeft(TreeNode@ node)
    {
        while (node !is null)
        {
            stack.insertLast(node);
            @node = node.left;
        }
    }

    TreeNode@ current()
    {
        if (stack.length() > 0)
            return stack[stack.length() - 1];
        return null;
    }

    void advance()
    {
        if (stack.length() > 0)
        {
            TreeNode@ node = stack[stack.length() - 1];
            stack.removeLast();
            if (node.right !is null)
                pushLeft(node.right);
        }
    }
}

class Tree
{
    private TreeNode@ root;

    // Foreach iterator protocol using custom iterator type
    TreeIterator@ opForBegin() const
    {
        return TreeIterator(root);
    }

    bool opForEnd(TreeIterator@ iterator) const
    {
        return iterator.current() is null;
    }

    TreeIterator@ opForNext(TreeIterator@ iterator) const
    {
        iterator.advance();
        return iterator;
    }

    int opForValue(TreeIterator@ iterator) const
    {
        return iterator.current().value;
    }
}

void example()
{
    Tree tree;
    // ... build tree ...

    // Foreach with custom iterator type
    foreach (int val : tree)
    {
        print("" + val);
    }
}
```

## Compilation Notes
- **Desugaring:** The foreach loop is compiled by transforming it into a for loop that uses the container's iterator protocol (`opForBegin`, `opForEnd`, `opForNext`, `opForValue`/`opForValue0`/`opForValue1`/etc.). The compilation pattern is:
  1. Evaluate the container expression once and store a reference (if needed).
  2. Call `container.opForBegin()` to obtain the initial iterator value.
  3. Enter a for loop that:
     - Checks `!container.opForEnd(iterator)` as the condition.
     - Calls `container.opForNext(iterator)` as the update expression.
     - Calls `container.opForValue(iterator)` (single variable) or `opForValue0`, `opForValue1`, etc. (multiple variables) at the start of each iteration to populate the iteration variables.
     - Executes the loop body.
- **Iterator protocol verification:** During semantic analysis, the compiler must verify that the container type implements the required methods:
  - `opForBegin()` returning any type (the iterator type).
  - `opForEnd(iterator_type)` returning `bool`.
  - `opForNext(iterator_type)` returning `iterator_type`.
  - `opForValue(iterator_type)` (single variable) or `opForValue0(iterator_type)`, `opForValue1(iterator_type)`, etc. (multiple variables) returning types compatible with the declared iteration variables.
  If any method is missing or has incompatible signatures, a compilation error is raised.
- **Iterator type:** The iterator type is inferred from the return type of `opForBegin()`. It can be a primitive (e.g., `int`), a value type (struct), or a handle. The iterator is stored in a local variable for the duration of the loop.
- **Variable scoping:** The iteration variables are scoped to the foreach statement. The compiler creates a scope encompassing the entire foreach construct. Variables must be destructed/released when the loop exits (normally or via `break`).
- **Break/continue targets:**
  - `break` jumps to the end of the foreach loop (after cleanup/destruction of iteration variables and iterator).
  - `continue` jumps to the update expression (`opForNext` call) and then re-evaluates the condition (`opForEnd`).
- **Stack behavior:**
  - The container reference (if an object) and iterator value are maintained on the stack across iterations.
  - Each iteration calls `opForValue`/`opForValue0`/`opForValue1`/etc., which may return by value (stack copy) or by handle (reference counting).
  - Iteration variables are assigned each iteration and must be cleaned up at loop exit.
- **Special cases:**
  - An empty container (`opForEnd` returns `true` immediately) results in zero iterations (the body is never executed).
  - Object-type iteration variables must be properly constructed/destructed each iteration. If `opForValue` returns by value, a copy is made.
  - Handle-type iteration variables must have their reference counts managed correctly (increment when assigned, decrement when reassigned or at loop exit).
  - The `opForValue`/`opForValue0`/`opForValue1`/etc. methods are called on every iteration, even if the iteration variable is not used in the body. This allows side effects in these methods.

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
