# Task 08i: Dictionary Module

**Status:** Not Started
**Parent:** Task 08 - Built-in Modules
**Depends On:** 08c (ScriptDict), 08d (modules), 08g (list behaviors)

---

## Objective

Register the `dictionary<K,V>` template type via FFI using the ClassBuilder API with all methods and list factory support.

## Files to Create/Modify

- `src/modules/dictionary.rs` - Dictionary template registration (new)
- Update `src/modules/mod.rs` - Add export

## Implementation

### src/modules/dictionary.rs

```rust
//! Dictionary template registration.
//!
//! Registers the built-in `dictionary<K,V>` template type with all methods and operators.

use crate::ffi::{Module, FfiRegistrationError, TemplateValidation, TemplateInstanceInfo, CallContext, NativeError};
use crate::runtime::{ScriptDict, ScriptKey, is_hashable_type};

/// Creates the dictionary module with the dictionary<K,V> template.
pub fn dictionary_module<'app>() -> Result<Module<'app>, FfiRegistrationError> {
    let mut module = Module::root();

    // =========================================================================
    // DICTIONARY<K,V> TEMPLATE REGISTRATION
    // =========================================================================

    module.register_type::<ScriptDict>("dictionary<class K, class V>")
        .reference_type()

        // Template validation - K must be hashable
        .template_callback(validate_dictionary_template)?

        // =====================================================================
        // FACTORIES
        // =====================================================================

        .factory("dictionary<K,V>@ f()", dict_new)?
        .factory("dictionary<K,V>@ f(uint capacity)", dict_with_capacity)?

        // List factory for initialization lists: dictionary@ d = {{"a", 1}, {"b", 2}}
        .list_factory("dictionary<K,V>@ f(int &in list) {repeat {K, V}}", dict_list_factory)?

        // Reference counting
        .addref(ScriptDict::add_ref)
        .release(ScriptDict::release)

        // =====================================================================
        // OPERATORS
        // =====================================================================

        .operator("dictionary<K,V>& opAssign(const dictionary<K,V> &in)", dict_assign)?
        .operator("V& opIndex(const K &in)", dict_index)?
        .operator("const V& opIndex(const K &in) const", dict_index_const)?

        // =====================================================================
        // SIZE AND CAPACITY
        // =====================================================================

        .method("uint getSize() const", dict_get_size)?
        .method("bool isEmpty() const", dict_is_empty)?
        .method("uint capacity() const", dict_capacity)?
        .method("void reserve(uint additional)", dict_reserve)?
        .method("void shrinkToFit()", dict_shrink_to_fit)?

        // =====================================================================
        // INSERTION AND UPDATE
        // =====================================================================

        .method("void set(const K &in key, const V &in value)", dict_set)?
        .method("bool insert(const K &in key, const V &in value)", dict_insert)?
        .method("V getOrInsert(const K &in key, const V &in default)", dict_get_or_insert)?
        .method("bool tryInsert(const K &in key, const V &in value)", dict_try_insert)?

        // =====================================================================
        // RETRIEVAL
        // =====================================================================

        .method("bool get(const K &in key, V &out value) const", dict_get)?
        .method("V getOr(const K &in key, const V &in default) const", dict_get_or)?
        .method("bool tryGet(const K &in key, V &out value) const", dict_try_get)?

        // =====================================================================
        // EXISTENCE AND DELETION
        // =====================================================================

        .method("bool exists(const K &in key) const", dict_exists)?
        .method("bool delete(const K &in key)", dict_delete)?
        .method("void deleteAll()", dict_delete_all)?
        .method("void clear()", dict_clear)?
        .method("V remove(const K &in key)", dict_remove)?
        .method("bool removeIf(const K &in key, const V &in expected)", dict_remove_if)?

        // =====================================================================
        // KEY/VALUE ACCESS
        // =====================================================================

        .method("array<K>@ getKeys() const", dict_get_keys)?
        .method("array<V>@ getValues() const", dict_get_values)?
        .method("array<K>@ keys() const", dict_keys)?        // Alias
        .method("array<V>@ values() const", dict_values)?    // Alias

        // =====================================================================
        // BULK OPERATIONS
        // =====================================================================

        .method("void extend(const dictionary<K,V> &in other)", dict_extend)?
        .method("dictionary<K,V>@ clone() const", dict_clone)?

        // =====================================================================
        // PREDICATES
        // =====================================================================

        .method("bool containsValue(const V &in value) const", dict_contains_value)?
        .method("uint countValue(const V &in value) const", dict_count_value)?

        .build()?;

    Ok(module)
}

// =============================================================================
// TEMPLATE VALIDATION
// =============================================================================

fn validate_dictionary_template(info: &TemplateInstanceInfo) -> TemplateValidation {
    // K must be hashable (primitives, string, handles)
    let key_type = &info.sub_types[0];

    if is_hashable_type(key_type) {
        TemplateValidation::valid()
    } else {
        TemplateValidation::invalid(
            "Dictionary key must be hashable (primitive, string, or handle)"
        )
    }
}

// =============================================================================
// FACTORY IMPLEMENTATIONS
// =============================================================================

fn dict_new(ctx: &mut CallContext) -> Result<(), NativeError> {
    let key_type = ctx.template_type(0)?;
    let value_type = ctx.template_type(1)?;
    let dict = ScriptDict::new(key_type, value_type);
    ctx.set_return_handle(dict)?;
    Ok(())
}

fn dict_with_capacity(ctx: &mut CallContext) -> Result<(), NativeError> {
    let capacity: u32 = ctx.arg(0)?;
    let key_type = ctx.template_type(0)?;
    let value_type = ctx.template_type(1)?;
    let dict = ScriptDict::with_capacity(key_type, value_type, capacity as usize);
    ctx.set_return_handle(dict)?;
    Ok(())
}

fn dict_list_factory(ctx: &mut CallContext) -> Result<(), NativeError> {
    let list = ctx.arg_tuple_list(0, 2)?; // Pairs of {K, V}
    let key_type = ctx.template_type(0)?;
    let value_type = ctx.template_type(1)?;

    let mut dict = ScriptDict::new(key_type, value_type);
    for pair in list.iter() {
        let key = ScriptKey::from_slot(&pair[0])
            .ok_or(NativeError::InvalidKey)?;
        let value = pair[1].clone();
        dict.insert(key, value);
    }

    ctx.set_return_handle(dict)?;
    Ok(())
}

// =============================================================================
// OPERATOR IMPLEMENTATIONS
// =============================================================================

fn dict_assign(ctx: &mut CallContext) -> Result<(), NativeError> {
    let other = ctx.arg_handle::<ScriptDict>(0)?;
    let this = ctx.this_mut::<ScriptDict>()?;
    this.clone_from(&other);
    ctx.set_return_ref(this)?;
    Ok(())
}

fn dict_index(ctx: &mut CallContext) -> Result<(), NativeError> {
    let key = ctx.arg_any(0)?;
    let script_key = ScriptKey::from_slot(&key)
        .ok_or(NativeError::InvalidKey)?;
    let this = ctx.this_mut::<ScriptDict>()?;

    // Get or insert default value
    let value = this.entry(script_key).or_insert_default();
    ctx.set_return_ref(value)?;
    Ok(())
}

fn dict_index_const(ctx: &mut CallContext) -> Result<(), NativeError> {
    let key = ctx.arg_any(0)?;
    let script_key = ScriptKey::from_slot(&key)
        .ok_or(NativeError::InvalidKey)?;
    let this = ctx.this_ref::<ScriptDict>()?;

    let value = this.get(&script_key)
        .ok_or(NativeError::KeyNotFound)?;
    ctx.set_return_ref(value)?;
    Ok(())
}

// =============================================================================
// SIZE AND CAPACITY IMPLEMENTATIONS
// =============================================================================

fn dict_get_size(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_ref::<ScriptDict>()?;
    ctx.set_return(this.len())?;
    Ok(())
}

fn dict_is_empty(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_ref::<ScriptDict>()?;
    ctx.set_return(this.is_empty())?;
    Ok(())
}

fn dict_capacity(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_ref::<ScriptDict>()?;
    ctx.set_return(this.capacity())?;
    Ok(())
}

fn dict_reserve(ctx: &mut CallContext) -> Result<(), NativeError> {
    let additional: u32 = ctx.arg(0)?;
    let this = ctx.this_mut::<ScriptDict>()?;
    this.reserve(additional as usize);
    Ok(())
}

fn dict_shrink_to_fit(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_mut::<ScriptDict>()?;
    this.shrink_to_fit();
    Ok(())
}

// =============================================================================
// INSERTION IMPLEMENTATIONS
// =============================================================================

fn dict_set(ctx: &mut CallContext) -> Result<(), NativeError> {
    let key = ctx.arg_any(0)?;
    let value = ctx.arg_any(1)?;
    let script_key = ScriptKey::from_slot(&key)
        .ok_or(NativeError::InvalidKey)?;
    let this = ctx.this_mut::<ScriptDict>()?;
    this.insert(script_key, value);
    Ok(())
}

fn dict_insert(ctx: &mut CallContext) -> Result<(), NativeError> {
    let key = ctx.arg_any(0)?;
    let value = ctx.arg_any(1)?;
    let script_key = ScriptKey::from_slot(&key)
        .ok_or(NativeError::InvalidKey)?;
    let this = ctx.this_mut::<ScriptDict>()?;

    // Returns false if key already existed
    let existed = this.insert(script_key, value).is_some();
    ctx.set_return(!existed)?;
    Ok(())
}

fn dict_get_or_insert(ctx: &mut CallContext) -> Result<(), NativeError> {
    let key = ctx.arg_any(0)?;
    let default = ctx.arg_any(1)?;
    let script_key = ScriptKey::from_slot(&key)
        .ok_or(NativeError::InvalidKey)?;
    let this = ctx.this_mut::<ScriptDict>()?;

    let value = this.entry(script_key).or_insert(default);
    ctx.set_return_any(value.clone())?;
    Ok(())
}

fn dict_try_insert(ctx: &mut CallContext) -> Result<(), NativeError> {
    let key = ctx.arg_any(0)?;
    let value = ctx.arg_any(1)?;
    let script_key = ScriptKey::from_slot(&key)
        .ok_or(NativeError::InvalidKey)?;
    let this = ctx.this_mut::<ScriptDict>()?;

    // Only insert if absent
    if this.contains_key(&script_key) {
        ctx.set_return(false)?;
    } else {
        this.insert(script_key, value);
        ctx.set_return(true)?;
    }
    Ok(())
}

// =============================================================================
// RETRIEVAL IMPLEMENTATIONS
// =============================================================================

fn dict_get(ctx: &mut CallContext) -> Result<(), NativeError> {
    let key = ctx.arg_any(0)?;
    let out_value = ctx.arg_out_ref(1)?;
    let script_key = ScriptKey::from_slot(&key)
        .ok_or(NativeError::InvalidKey)?;
    let this = ctx.this_ref::<ScriptDict>()?;

    match this.get(&script_key) {
        Some(value) => {
            out_value.copy_from(value);
            ctx.set_return(true)?;
        }
        None => {
            ctx.set_return(false)?;
        }
    }
    Ok(())
}

fn dict_get_or(ctx: &mut CallContext) -> Result<(), NativeError> {
    let key = ctx.arg_any(0)?;
    let default = ctx.arg_any(1)?;
    let script_key = ScriptKey::from_slot(&key)
        .ok_or(NativeError::InvalidKey)?;
    let this = ctx.this_ref::<ScriptDict>()?;

    let result = this.get(&script_key).cloned().unwrap_or(default);
    ctx.set_return_any(result)?;
    Ok(())
}

fn dict_try_get(ctx: &mut CallContext) -> Result<(), NativeError> {
    // Same as dict_get - just an alias
    dict_get(ctx)
}

// =============================================================================
// EXISTENCE AND DELETION IMPLEMENTATIONS
// =============================================================================

fn dict_exists(ctx: &mut CallContext) -> Result<(), NativeError> {
    let key = ctx.arg_any(0)?;
    let script_key = ScriptKey::from_slot(&key)
        .ok_or(NativeError::InvalidKey)?;
    let this = ctx.this_ref::<ScriptDict>()?;
    ctx.set_return(this.contains_key(&script_key))?;
    Ok(())
}

fn dict_delete(ctx: &mut CallContext) -> Result<(), NativeError> {
    let key = ctx.arg_any(0)?;
    let script_key = ScriptKey::from_slot(&key)
        .ok_or(NativeError::InvalidKey)?;
    let this = ctx.this_mut::<ScriptDict>()?;
    let existed = this.remove(&script_key).is_some();
    ctx.set_return(existed)?;
    Ok(())
}

fn dict_delete_all(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_mut::<ScriptDict>()?;
    this.clear();
    Ok(())
}

fn dict_clear(ctx: &mut CallContext) -> Result<(), NativeError> {
    // Alias for deleteAll
    dict_delete_all(ctx)
}

fn dict_remove(ctx: &mut CallContext) -> Result<(), NativeError> {
    let key = ctx.arg_any(0)?;
    let script_key = ScriptKey::from_slot(&key)
        .ok_or(NativeError::InvalidKey)?;
    let this = ctx.this_mut::<ScriptDict>()?;
    let value = this.remove(&script_key)
        .ok_or(NativeError::KeyNotFound)?;
    ctx.set_return_any(value)?;
    Ok(())
}

fn dict_remove_if(ctx: &mut CallContext) -> Result<(), NativeError> {
    let key = ctx.arg_any(0)?;
    let expected = ctx.arg_any(1)?;
    let script_key = ScriptKey::from_slot(&key)
        .ok_or(NativeError::InvalidKey)?;
    let this = ctx.this_mut::<ScriptDict>()?;

    // Only remove if value matches
    match this.get(&script_key) {
        Some(value) if value.eq_slot(&expected) => {
            this.remove(&script_key);
            ctx.set_return(true)?;
        }
        _ => {
            ctx.set_return(false)?;
        }
    }
    Ok(())
}

// =============================================================================
// KEY/VALUE ACCESS IMPLEMENTATIONS
// =============================================================================

fn dict_get_keys(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_ref::<ScriptDict>()?;
    let key_type = ctx.template_type(0)?;

    let mut arr = ScriptArray::new(key_type);
    for key in this.keys() {
        arr.push(key.to_slot());
    }

    ctx.set_return_handle(arr)?;
    Ok(())
}

fn dict_get_values(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_ref::<ScriptDict>()?;
    let value_type = ctx.template_type(1)?;

    let mut arr = ScriptArray::new(value_type);
    for value in this.values() {
        arr.push(value.clone());
    }

    ctx.set_return_handle(arr)?;
    Ok(())
}

fn dict_keys(ctx: &mut CallContext) -> Result<(), NativeError> {
    // Alias for getKeys
    dict_get_keys(ctx)
}

fn dict_values(ctx: &mut CallContext) -> Result<(), NativeError> {
    // Alias for getValues
    dict_get_values(ctx)
}

// =============================================================================
// BULK OPERATIONS IMPLEMENTATIONS
// =============================================================================

fn dict_extend(ctx: &mut CallContext) -> Result<(), NativeError> {
    let other = ctx.arg_handle::<ScriptDict>(0)?;
    let this = ctx.this_mut::<ScriptDict>()?;
    this.extend(&other);
    Ok(())
}

fn dict_clone(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_ref::<ScriptDict>()?;
    let cloned = this.clone_dict();
    ctx.set_return_handle(cloned)?;
    Ok(())
}

// =============================================================================
// PREDICATE IMPLEMENTATIONS
// =============================================================================

fn dict_contains_value(ctx: &mut CallContext) -> Result<(), NativeError> {
    let value = ctx.arg_any(0)?;
    let this = ctx.this_ref::<ScriptDict>()?;
    ctx.set_return(this.contains_value(&value))?;
    Ok(())
}

fn dict_count_value(ctx: &mut CallContext) -> Result<(), NativeError> {
    let value = ctx.arg_any(0)?;
    let this = ctx.this_ref::<ScriptDict>()?;
    ctx.set_return(this.count_value(&value))?;
    Ok(())
}
```

## Method Summary

| Category | Methods |
|----------|---------|
| **Factories** | f(), f(uint), list factory |
| **Lifecycle** | addref, release |
| **Operators** | opAssign, opIndex (const + mutable) |
| **Size** | getSize, isEmpty, capacity, reserve, shrinkToFit |
| **Insert** | set, insert, getOrInsert, tryInsert |
| **Get** | get, getOr, tryGet |
| **Delete** | exists, delete, deleteAll, clear, remove, removeIf |
| **Keys/Values** | getKeys, getValues, keys, values |
| **Bulk** | extend, clone |
| **Predicates** | containsValue, countValue |

## Template Validation

The dictionary template validates that K is hashable:
- Primitive types (int, float, bool, etc.) - OK
- String - OK
- Handles (@) - OK
- Value types without opHash - INVALID

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dictionary_module_builds() {
        let module = dictionary_module().expect("dictionary module should build");
        // Should have dictionary template registered
        assert!(module.types().iter().any(|t| t.name() == "dictionary"));
    }

    #[test]
    fn test_validate_hashable_key() {
        // int is hashable
        assert!(is_hashable_type(TypeId::INT32));
        // string is hashable
        assert!(is_hashable_type(TypeId::STRING_TYPE));
        // void is not hashable
        assert!(!is_hashable_type(TypeId::VOID_TYPE));
    }
}
```

## Acceptance Criteria

- [ ] `src/modules/dictionary.rs` created
- [ ] Dictionary template registered as reference type
- [ ] All factories registered including list factory
- [ ] AddRef/Release behaviors registered
- [ ] All operators registered (3)
- [ ] All methods registered (~20)
- [ ] Template callback validates K is hashable
- [ ] Unit tests pass
- [ ] `cargo build --lib` succeeds
