# Advanced Data Types

## Strings

**Note:** Strings must be registered by the application. Syntax may vary.

### String Literals

```angelscript
// Normal strings (double or single quotes)
string str1 = "This is a string with \"escape sequences\".";
string str2 = 'Single quotes allow double quotes without escaping.';

// Heredoc strings (no escape processing)
string str = """
This is some text without "escape sequences".
Multi-line content is preserved.
""";
```

### Escape Sequences

| Sequence | Value | Description |
|----------|-------|-------------|
| `\0` | 0 | Null character |
| `\\` | 92 | Backslash |
| `\'` | 39 | Single quote |
| `\"` | 34 | Double quote |
| `\n` | 10 | Newline |
| `\r` | 13 | Carriage return |
| `\t` | 9 | Tab |
| `\xFFFF` | 0xFFFF | Hex value (1-4 digits) |
| `\uFFFF` | Unicode | UTF-8/UTF-16 code point |
| `\UFFFFFFFF` | Unicode | Full 32-bit code point |

### String Concatenation

Adjacent string literals are concatenated:

```angelscript
string str = "First line.\n"
             "Second line.\n"
             "Third line.\n";
```

## Arrays

**Note:** Arrays must be registered by the application.

### Declaration

```angelscript
array<int> a, b, c;      // Arrays of integers
array<Foo@> d;           // Array of handles

array<int> a;            // Zero-length
array<int> b(3);         // Length 3, default values
array<int> c(3, 1);      // Length 3, all initialized to 1
array<int> d = {5,6,7};  // Initialization list
```

### Multidimensional Arrays

```angelscript
array<array<int>> a;                      // Empty 2D array
array<array<int>> b = {{1,2},{3,4}};      // 2x2 with values
array<array<int>> c(10, array<int>(10));  // 10x10
```

### Access

```angelscript
a[0] = value;            // Index access (0-based)
@arr[0] = Foo();         // Handle assignment in handle array
```

### Array Methods

| Method | Description |
|--------|-------------|
| `uint length()` | Get array length |
| `void resize(uint)` | Set new length |
| `void reverse()` | Reverse element order |
| `void insertAt(uint idx, T val)` | Insert at index |
| `void insertLast(T val)` | Append element |
| `void removeAt(uint idx)` | Remove at index |
| `void removeLast()` | Remove last element |
| `void removeRange(uint start, uint count)` | Remove range |
| `void sortAsc()` | Sort ascending |
| `void sortDesc()` | Sort descending |
| `void sort(callback)` | Sort with custom comparison |
| `int find(T val)` | Find by value (-1 if not found) |
| `int findByRef(T val)` | Find by reference |

### Array Operators

- `=` - Shallow copy
- `[]` - Index access (exception if out of range)
- `==`, `!=` - Value comparison of all elements

### Anonymous Array Construction

```angelscript
foo({1,2,3,4});                    // Implicit array type
foo(array<int> = {1,2,3,4});       // Explicit type (for overloads)
```

## Auto Type

Type inference for variable declarations:

```angelscript
auto i = 18;           // int
auto f = 18 + 5.f;     // float
auto o = getObject();  // Inferred from return type
```

### Auto Rules

- Requires initialization expression
- `const auto` forces constant
- For reference types, `auto` becomes a **handle** (more efficient)
- Cannot be used for class members (depends on constructor)

```angelscript
auto  a = getObject();  // a is obj@
auto@ b = getObject();  // Explicit handle syntax (same result)
```

## Function Handles

Store pointers to functions with matching signatures.

### Funcdef Declaration

```angelscript
funcdef bool CALLBACK(int, int);
```

### Usage

```angelscript
// Assign function to handle
CALLBACK@ func = @myCompare;

// Check for null
if (func is null) { return; }

// Call through handle
bool result = func(1, 2);

// Matching function
bool myCompare(int a, int b) {
    return a > b;
}
```

### Delegates (Method Handles)

Bind a class method to an object instance:

```angelscript
class A {
    bool Cmp(int a, int b) { return a > b; }
}

void main() {
    A a;
    CALLBACK@ func = CALLBACK(a.Cmp);  // Create delegate
    func(1, 2);                         // Calls a.Cmp(1, 2)
}
```

## Anonymous Functions (Lambdas)

Inline function definitions for use with function handles:

```angelscript
funcdef bool CMP(int, int);

void main() {
    bool result = func(1, 2, function(a,b) { return a == b; });
}

bool func(int a, int b, CMP@ f) {
    return f(a, b);
}
```

### Lambda Rules

- Signature inferred from target funcdef
- Parameter and return types can be omitted
- **Cannot capture variables** from enclosing scope (not closures)

### Explicit Types for Ambiguous Cases

```angelscript
funcdef void A(int);
funcdef void B(float);
void func(A@) {}
void func(B@) {}

void main() {
    func(function(int a) {});  // Explicitly specify int
}
```

## Dictionary

**Note:** Must be registered by application.

```angelscript
dictionary d;
d["key"] = 42;
int val = int(d["key"]);

// Initialization
dictionary d2 = {{"key1", 1}, {"key2", "value"}};
```

## Ref Type

**Note:** Must be registered by application.

Generic reference that can hold any reference type:

```angelscript
ref@ r = someObject;
MyClass@ obj = cast<MyClass>(r);
```

## Weak References

**Note:** Must be registered by application.

Reference that doesn't prevent object destruction:

```angelscript
weakref<MyClass> w = obj;   // Create weak reference
MyClass@ strong = w.get();  // Get strong reference (null if destroyed)
```
