# Dictionary

## Overview

The `dictionary` type is a dynamic key-value container where keys are always strings and values can be of any type. Key-value pairs can be added and removed at runtime, making the dictionary a versatile general-purpose container. Dictionaries are only available if the host application registers support for them. The dictionary is a reference type, so handles can be used to pass it efficiently.

## Syntax

### Declaration and initialization

```angelscript
// Empty dictionary
dictionary d;

// Initialization list
dictionary dict = {{'one', 1}, {'object', object}, {'handle', @handle}};

// Nested dictionaries
dictionary d2 = {
    {'a', dictionary = {{'aa', 1}, {'ab', 2}}},
    {'b', dictionary = {{'ba', 1}, {'bb', 2}}}
};
```

### Access via methods

```angelscript
// Set values
dict.set('key', 42);
dict.set('name', "hello");

// Get values (returns bool indicating success)
int val;
bool found = dict.get('key', val);

// Get handle
obj@ handle;
bool ok = dict.get('handle', @handle);

// Check existence
if (dict.exists('key')) { }

// Delete a key
bool deleted = dict.delete('key');

// Delete all entries
dict.deleteAll();

// Check if empty
if (dict.isEmpty()) { }

// Get number of entries
uint count = dict.getSize();

// Get all keys
array<string>@ keys = dict.getKeys();
```

### Access via index operator

```angelscript
// Read a value (requires explicit cast/conversion)
int val = int(dict['value']);

// Write a value
dict['value'] = val + 1;

// Read a handle
@handle = cast<obj>(dict['handle']);

// Write a handle
@dict['handle'] = object;
```

### Anonymous dictionary construction

```angelscript
// Implicit type (unambiguous overload)
foo({{'a', 1}, {'b', 2}});

// Explicit type (for disambiguation)
foo2(dictionary = {{'a', 1}, {'b', 2}});
```

## Semantics

### Dictionary object

The dictionary is a **reference type**. Handles can be used to avoid copies.

**Operators:**

| Operator | Description |
|----------|-------------|
| `=` | Shallow copy of all key-value pairs |
| `[]` | Index by string key; returns a reference to a `dictionaryValue`. If the key does not exist, it is inserted with a null value. |

**Methods:**

| Method | Description |
|--------|-------------|
| `void set(const string &in key, ? &in value)` | Sets a key-value pair (generic value) |
| `void set(const string &in key, int64 &in value)` | Sets a key-value pair (int64) |
| `void set(const string &in key, double &in value)` | Sets a key-value pair (double) |
| `bool get(const string &in key, ? &out value) const` | Gets a value by key; returns false if key not found |
| `bool get(const string &in key, int64 &out value) const` | Gets an int64 value |
| `bool get(const string &in key, double &out value) const` | Gets a double value |
| `array<string>@ getKeys() const` | Returns an array of all keys (order undefined) |
| `bool exists(const string &in key) const` | Returns true if the key exists |
| `bool delete(const string &in key)` | Removes a key-value pair; returns false if key not found |
| `void deleteAll()` | Removes all entries |
| `bool isEmpty() const` | Returns true if dictionary has no entries |
| `uint getSize() const` | Returns the number of key-value pairs |

### dictionaryValue object

The `dictionaryValue` type is how the dictionary internally stores values. It is returned by the index operator and is a **value type** (no handles to it can be held). However, it can hold handles to other objects as well as values of any type.

**Operators:**

| Operator | Description |
|----------|-------------|
| `=` | Value assignment: copies a value into the dictionaryValue |
| `@=` | Handle assignment: sets the dictionaryValue to hold a handle to an object |
| `cast<type>` | Dynamic cast: returns a handle of the requested type, or null if incompatible |
| `type()` | Conversion operator: returns a new value of the requested type, or uninitialized default if no conversion exists |

### Type-agnostic storage

The dictionary stores values in a type-erased manner. The `set` and `get` methods have overloads for the three fundamental storage categories:
1. **Generic (`?`)**: Any type, stored as a type-erased blob.
2. **int64**: Integer values stored efficiently.
3. **double**: Floating-point values stored efficiently.

When retrieving a value, the type must be compatible with the stored type. The `get` method returns `false` if the types are incompatible.

## Examples

```angelscript
// Basic usage
dictionary config;
config.set('width', 1920);
config.set('height', 1080);
config.set('title', "My App");

int width;
if (config.get('width', width)) {
    print("Width: " + width + "\n");
}

// Index operator usage
config['fullscreen'] = true;
bool fs = bool(config['fullscreen']);

// Iteration over keys
array<string>@ keys = config.getKeys();
for (uint i = 0; i < keys.length(); i++) {
    print("Key: " + keys[i] + "\n");
}

// Storing object handles
class Player {
    string name;
}

Player p;
p.name = "Alice";

dictionary registry;
@registry['player1'] = @p;

Player@ retrieved;
registry.get('player1', @retrieved);
print(retrieved.name + "\n");   // "Alice"

// Nested dictionaries
dictionary outer = {
    {'settings', dictionary = {
        {'volume', 75},
        {'muted', false}
    }},
    {'user', dictionary = {
        {'name', "Bob"},
        {'level', 5}
    }}
};

// Anonymous construction in function call
void configure(dictionary@ opts) { }
configure(dictionary = {{'debug', true}, {'verbose', false}});
```

## Compilation Notes

- **Memory layout:** The dictionary is a heap-allocated reference type containing a reference count and an internal hash map (or equivalent) mapping `string` keys to `dictionaryValue` entries. Each `dictionaryValue` stores a type tag, a union/variant for the value (int64, double, or a type-erased object pointer with type info), and for handle values, the handle pointer with refcount management.
- **Stack behavior:** A `dictionary` variable on the stack is a pointer to the heap object (one pointer-sized slot). Handles to dictionaries follow normal handle semantics (addref/release).
- **Type considerations:**
  - The index operator `[]` returns a reference to a `dictionaryValue`, not the stored value directly. The caller must then use conversion operators (`int()`, `cast<T>()`) to extract the actual value.
  - The `set`/`get` methods with the `?` (any-type) parameter use runtime type information to store and retrieve values. The compiler must emit type info alongside the value.
  - The three `set`/`get` overloads (generic, int64, double) exist for efficiency: integers and doubles can be stored inline without boxing.
  - Storing a handle via `@dict['key'] = obj` uses handle assignment on the dictionaryValue, which stores the pointer and increments the refcount.
- **Lifecycle:**
  - Construction: Allocate dictionary on heap (refcount = 1). Initialize empty hash map.
  - Initialization list: After construction, insert each key-value pair. For handle values in the init list, use handle assignment.
  - Assignment (`=`): Shallow copy of all entries. For handle values, addref each. For value entries, copy the value.
  - Destruction: When refcount reaches zero, iterate all entries, release all handle values, destroy all value entries, free the hash map, free the dictionary object.
  - `delete()`: Remove the entry, release/destroy its value.
  - `deleteAll()`: Release/destroy all values, clear the hash map.
- **Special cases:**
  - Accessing a non-existent key via `[]` inserts a null entry. This differs from `get()`, which returns false without modifying the dictionary.
  - The `getKeys()` method allocates and returns a new `array<string>` object. The caller receives a handle to it.
  - Anonymous dictionary construction in expressions (`dictionary = {{...}}`) creates a temporary that must be properly reference-counted.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/types.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `TypeBase::Named(Ident { name: "dictionary", .. })` | Dictionary type as a named type | Wraps `Ident` with `name = "dictionary"` |

**Notes:**
- `dictionary` is **not** a parser-level built-in. It is a runtime-registered type provided by the host application. The parser sees it as `TypeBase::Named(Ident { name: "dictionary", .. })` with no template arguments (unlike `array<T>`, the dictionary is not a template type).
- The `dictionaryValue` type is also an application-registered type and would appear as `TypeBase::Named(Ident { name: "dictionaryValue", .. })` if used explicitly in script code.
- Dictionary initialization lists (e.g., `{{'key', value}, ...}`) are expression-level constructs, not part of the type AST.
- `dictionary@` (handle to dictionary) uses `TypeExpr.suffixes: &[TypeSuffix::Handle { is_const: false }]`.
- The parser does not validate that `dictionary` is a valid registered type; that is deferred to semantic analysis.

## Related Features

- [Strings (dictionary keys)](./strings.md)
- [Arrays (getKeys returns array)](./arrays.md)
- [Handles (storing object handles in dictionaries)](./handles.md)
- [Objects (stored value types)](./objects.md)
