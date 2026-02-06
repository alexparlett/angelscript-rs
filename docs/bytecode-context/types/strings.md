# Strings

## Overview

The `string` type in AngelScript holds an array of bytes (or 16-bit words depending on application settings). Strings are typically used for text but can store arbitrary binary data. Strings are only available if the host application registers support for them. The `string` type is a reference type in terms of registration, though it often behaves like a value type in practice (application-dependent). The language supports three forms of string literals: double-quoted, single-quoted, and heredoc (triple-quoted).

## Syntax

### String literals

```angelscript
// Double-quoted string (escape sequences processed)
string str1 = "Hello, world!\n";

// Single-quoted string (escape sequences processed, but double quotes need no escaping)
string str2 = 'She said "hello" to everyone.';

// Heredoc string (no escape processing, preserves content literally)
string str3 = """
This is a heredoc string.
It can span multiple lines.
No "escape sequences" are processed: \n is literal.
""";
```

### Escape sequences

| Sequence | Value | Description |
|----------|-------|-------------|
| `\0` | 0 | Null character |
| `\\` | 92 | Backslash |
| `\'` | 39 | Single quotation mark (apostrophe) |
| `\"` | 34 | Double quotation mark |
| `\n` | 10 | New line feed |
| `\r` | 13 | Carriage return |
| `\t` | 9 | Tab character |
| `\xFFFF` | 0xFFFF | Hexadecimal value (1 to 4 hex digits). For 8-bit strings, max value is 255. |
| `\uFFFF` | Unicode | Unicode code point encoded as UTF-8 or UTF-16 (depending on application). |
| `\UFFFFFFFF` | Unicode | Full 32-bit Unicode code point. |

The `\u` and `\U` escape sequences only accept valid Unicode 5.1 code points. Code points between U+D800 and U+DFFF (surrogate pair range) and code points above U+10FFFF are rejected.

### String concatenation at compile time

Adjacent string literals separated only by whitespace or comments are concatenated by the compiler into a single constant:

```angelscript
string str = "First line.\n"
             "Second line.\n"
             "Third line.\n";
// Equivalent to: "First line.\nSecond line.\nThird line.\n"
```

### Heredoc whitespace trimming

For heredoc strings:
- If the characters after the opening `"""` until the first linebreak contain only whitespace, that leading whitespace line is removed.
- If the characters after the last linebreak until the closing `"""` contain only whitespace, that trailing whitespace line is removed.

```angelscript
string str = """
Content starts here.
Last line of content.
""";
// The leading and trailing whitespace-only lines are stripped.
```

## Semantics

- Strings hold a sequence of bytes or 16-bit words (application-configured).
- String comparison operators (`==`, `!=`, `<`, `>`, `<=`, `>=`) perform lexicographic comparison.
- The `+` operator concatenates two strings, returning a new string.
- The `+=` operator appends to the existing string.
- Index operator `[]` accesses individual characters (bytes) by index.
- The `length()` method returns the number of characters/bytes.
- Strings can be implicitly constructed from primitives in certain contexts (e.g., `"value: " + intVar`).
- Strings are typically passed by reference (`const string &in`) for efficiency.

### Common methods (standard add-on)

| Method | Description |
|--------|-------------|
| `uint length() const` | Returns the length of the string |
| `void resize(uint)` | Sets the length of the string |
| `bool isEmpty() const` | Returns true if the string is empty |
| `string substr(uint start, int count = -1) const` | Returns a substring |
| `int findFirst(const string &in str, uint start = 0) const` | Finds first occurrence |
| `int findLast(const string &in str, int start = -1) const` | Finds last occurrence |
| `void insert(uint pos, const string &in other)` | Inserts a string at position |
| `void erase(uint pos, int count = -1)` | Erases characters |

### Common operators

| Operator | Description |
|----------|-------------|
| `=` | Value assignment (copies the string) |
| `+` | Concatenation (returns new string) |
| `+=` | Append |
| `==`, `!=` | Equality/inequality comparison |
| `<`, `>`, `<=`, `>=` | Lexicographic comparison |
| `[]` | Character index access |

## Examples

```angelscript
// Basic string operations
string greeting = "Hello";
string name = "World";
string message = greeting + ", " + name + "!";  // "Hello, World!"

// Escape sequences
string path = "C:\\Users\\file.txt";
string tab = "col1\tcol2\tcol3";
string newline = "line1\nline2";

// Single-quoted string
string quoted = 'He said "yes" immediately.';

// Heredoc for multi-line content
string json = """
{
    "name": "test",
    "value": 42
}
""";

// String comparison
if (greeting == "Hello") {
    print("Match!\n");
}

// Index access
string s = "ABCDE";
uint8 ch = s[0];   // 65 (ASCII 'A')

// Concatenation of adjacent literals
string multipart = "Part 1 "
                   "Part 2 "
                   "Part 3";
// Result: "Part 1 Part 2 Part 3"

// Integer to string concatenation
int value = 42;
string result = "The answer is " + value;
```

## Compilation Notes

- **Memory layout:** Strings are typically registered as a value type with the `asOBJ_APP_CLASS_CDAK` flags, backed by a C++ `std::string` or equivalent. On the stack, a string variable holds the full string object (not a pointer). The exact size depends on the application's string class registration.
- **Stack behavior:** When registered as a value type, strings are stored directly on the stack. When passed as `const string &in`, only a reference (pointer) is passed. The size of a string on the stack matches the registered type's size (usually `sizeof(std::string)`).
- **Type considerations:**
  - String literals are stored in the module's constant data segment. At runtime, a string constant is constructed from the constant data.
  - Implicit conversion from primitives to string (e.g., `"value: " + intVar`) requires the application to register appropriate `opAdd` / `opImplConv` methods.
  - Unicode escape sequences (`\u`, `\U`) are resolved at compile time into the appropriate byte sequence.
  - Adjacent literal concatenation is performed at compile time, producing a single constant.
- **Lifecycle:**
  - Construction: The string constructor is called (from a literal or default).
  - Copy: Assignment calls the string's `opAssign`, which copies the content.
  - Destruction: The destructor is called when the variable goes out of scope, freeing the internal buffer.
  - Temporary strings from concatenation must be properly constructed, used, and destroyed within the expression.
- **Special cases:**
  - Heredoc strings skip escape sequence processing entirely; the raw text between the `"""` delimiters (after whitespace trimming) becomes the constant.
  - The compiler must handle compile-time concatenation of adjacent literals by merging them before emitting the constant.
  - Index operator `[]` may return a reference to a byte or a copy depending on the registration. Out-of-bounds access behavior depends on the application's implementation.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/types.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `TypeBase::Named(Ident { name: "string", .. })` | String type as a named type | Wraps `Ident` with `name = "string"` |

**Notes:**
- `string` is **not** a parser-level primitive. It is a runtime-registered type provided by the host application. The parser sees it as `TypeBase::Named(Ident { name: "string", .. })`, identical to any other user-defined or application-registered type.
- There is no `PrimitiveType::String` variant. The distinction between `string` and other named types is made during semantic analysis when the type registry is consulted.
- String literals (double-quoted, single-quoted, heredoc) are expression-level constructs handled by the expression AST, not the type AST.
- Passing a string by const reference (`const string &in`) is represented as `ParamType { ty: TypeExpr { is_const: true, base: TypeBase::Named("string"), .. }, ref_kind: RefKind::RefIn, .. }`.

## Related Features

- [Primitive types (string conversion from primitives)](./primitives.md)
- [Arrays (array of strings)](./arrays.md)
- [Dictionary (string keys)](./dictionary.md)
