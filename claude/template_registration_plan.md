# Template Parameter Representation - Design Document

**Status:** ✅ Implemented (Phase 6)

## Summary

Unify template parameter storage to use `Vec<TypeHash>` everywhere, with `TemplateParamEntry` registered for each parameter. This matches how AngelScript C++ stores template subtypes as `asCDataType` (type objects, not just names).

## How AngelScript C++ Does It

From `reference/angelscript/source/as_scriptengine.cpp` lines 1858-1884:

```cpp
// Template registration creates subtype entries:
for( asUINT subTypeIdx = 0; subTypeIdx < subtypeNames.GetLength(); subTypeIdx++ )
{
    // Look for existing subtype by NAME
    asCTypeInfo *subtype = 0;
    for( asUINT n = 0; n < templateSubTypes.GetLength(); n++ )
    {
        if( templateSubTypes[n]->name == subtypeNames[subTypeIdx] )
        {
            subtype = templateSubTypes[n];
            break;
        }
    }
    if( subtype == 0 )
    {
        // Create new subtype entry with asOBJ_TEMPLATE_SUBTYPE flag
        subtype = asNEW(asCTypeInfo)(this);
        subtype->name = subtypeNames[subTypeIdx];
        subtype->flags = asOBJ_TEMPLATE_SUBTYPE;
        templateSubTypes.PushLast(subtype);
    }
    // Store as asCDataType (type reference), NOT as string
    type->templateSubTypes.PushLast(asCDataType::CreateType(subtype, false));
}
```

Key insight: **Template params are stored as type references**, not names. The names are just used for initial lookup/creation.

## Data Structures

| Structure | Field | Type | Purpose |
|-----------|-------|------|---------|
| `ClassMeta` | `template_params` | `Vec<&'static str>` | Input from macros (names) |
| `ClassEntry` | `template_params` | `Vec<TypeHash>` | Hashes to TemplateParamEntry |
| `FunctionMeta` | `template_params` | `Vec<&'static str>` | Input from macros (names) |
| `FunctionDef` | `template_params` | `Vec<TypeHash>` | Hashes to TemplateParamEntry |
| `TemplateParamEntry` | - | struct | Registry entry for each param |

## Implementation

### Class Registration (`context.rs` - `install_class()`)

```rust
// Register template parameters as TemplateParamEntry and collect their hashes
if !meta.template_params.is_empty() {
    let mut template_param_hashes = Vec::with_capacity(meta.template_params.len());
    for (index, param_name) in meta.template_params.iter().enumerate() {
        let param_entry = TemplateParamEntry::for_template(
            *param_name,           // name: "T"
            index,                 // index: 0, 1, 2...
            meta.type_hash,        // owner: hash of the template class
            &qualified_name,       // owner_name: "MyClass" or "ns::MyClass"
        );
        let param_hash = param_entry.type_hash;
        template_param_hashes.push(param_hash);

        // Register the TemplateParamEntry in the registry
        self.registry.register_type(param_entry.into())?;
    }
    class_entry = class_entry.with_template_params(template_param_hashes);
}
```

### Function Registration (`context.rs` - `install_function()`)

```rust
// Register template parameters as TemplateParamEntry and collect their hashes
let mut template_param_hashes = Vec::with_capacity(meta.template_params.len());
for (index, param_name) in meta.template_params.iter().enumerate() {
    let param_entry = TemplateParamEntry::for_template(
        *param_name,           // name: "T"
        index,                 // index: 0
        func_hash,             // owner: hash of the function
        &qualified_func_name,  // owner_name: "MyFunc" or "ns::MyFunc"
    );
    let param_hash = param_entry.type_hash;
    template_param_hashes.push(param_hash);

    // Register the TemplateParamEntry in the registry
    self.registry.register_type(param_entry.into())?;
}
```

### Hash Naming Convention

Template parameter hashes follow the pattern `owner_name::param_name`:

| Template | Parameter | Hash Name |
|----------|-----------|-----------|
| `array<T>` | T | `array::T` |
| `dict<K, V>` | K | `dict::K` |
| `dict<K, V>` | V | `dict::V` |
| `std::vector<T>` | T | `std::vector::T` |
| `identity<T>` (function) | T | `identity::T` |

---

## Function Templates - Three Cases

### Case 1: Methods of Template Types (no own params)

Methods that only use the **owner's** template params:

```cpp
// array<T> has method: void push(const T &in)
// dict<K, V> has method: void insert(const K &in, const V &in)
```

- `FunctionDef.template_params` = **empty**
- Param types reference parent's `TemplateParamEntry`: `DataType::simple(TypeHash::from_name("dict::K"))`

### Case 2: Standalone Template Functions

Global functions with their own template parameters:

```cpp
// T Test<T, U>(T t, U u)
```

- `FunctionDef.template_params` = function's own param hashes
- `TemplateParamEntry::for_template("T", 0, func_hash, "Test")`

### Case 3: Template Methods with Mixed Params

Methods on template classes that have **both** parent params AND their own:

```cpp
// array<T> has template method: U Map<U>(Func<T, U> mapper)
// T comes from array<T>, U comes from the method itself
```

- `FunctionDef.object_type` = parent template class hash
- `FunctionDef.template_params` = method's own params only (`U`)
- Param types can reference **either**:
  - Parent's params: `TypeHash::from_name("array::T")`
  - Method's own params: `TypeHash::from_name("array::Map::U")`

**Registration:**
```rust
// For array<T>::Map<U>
let method_param = TemplateParamEntry::for_template(
    "U",                    // name
    0,                      // index (within the method)
    func_hash,              // owner = method hash
    "array::Map",           // owner_name = qualified method name
);
```

### Key Differences

| Aspect | Case 1: Type Methods | Case 2: Global Funcs | Case 3: Mixed |
|--------|---------------------|---------------------|---------------|
| `object_type` | Some(class) | None | Some(template class) |
| `template_params` | Empty | Own params | Own params only |
| Param type refs | Parent's only | Own only | Both parent's + own |

### Summary

All three cases are supported. The `FunctionDef.template_params` field contains only the **function's own** template params (if any), never the parent's.

---

## Benefits

1. **Consistency**: Both ClassEntry and FunctionDef use `Vec<TypeHash>`
2. **Type safety**: Template params are proper type references, not strings
3. **Lookup efficiency**: Can look up param info by hash directly
4. **Matches C++ design**: AngelScript stores template subtypes as type references
5. **Enables instantiation**: During template instantiation, params can be looked up by hash and substituted

## Compiler Considerations

When the compiler processes template types and functions:

1. **Type Resolution**: When encountering a type like `T` in a template context, resolve it by:
   - Looking up `owner_name::T` in the registry
   - Verify it's a `TemplateParamEntry`

2. **Template Instantiation**: When instantiating `array<int>`:
   - Look up `array::T` to get the template param
   - Create substitution map: `array::T` → `int`
   - Apply substitutions to all param types and return type

3. **Mixed Params (Case 3)**: When resolving types in `array<T>::Map<U>`:
   - First check method's own params (`array::Map::U`)
   - Then check parent's params (`array::T`)
