# Task 08c: Runtime Dictionary Type

**Status:** Not Started
**Parent:** Task 08 - Built-in Modules
**Depends On:** 08a, 08b (runtime module and VmSlot comparison)

---

## Objective

Create `ScriptDict` - a type-erased, reference-counted dictionary type that backs the AngelScript `dictionary<K,V>` template.

## Files to Modify

- `src/runtime/mod.rs` - Add export
- `src/runtime/dict.rs` - ScriptDict implementation (new)

## Design Considerations

### Hashable Keys

HashMap requires keys to implement `Hash + Eq`. VmSlot contains non-hashable types like `f64`. We need a wrapper type `ScriptKey` that:
1. Only accepts hashable types (primitives, strings, handles)
2. Implements `Hash + Eq` appropriately

### Floating Point Keys

AngelScript allows float keys (unlike Rust). We use `ordered_float::OrderedFloat` to make floats hashable and comparable.

### Reference Counting

Like arrays, dictionaries are reference types with manual reference counting.

## Implementation

```rust
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use ordered_float::OrderedFloat;

/// Hashable wrapper for dictionary keys.
/// Only primitives, strings, and handles are valid keys.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScriptKey {
    Int(i64),
    Float(OrderedFloat<f64>),
    Bool(bool),
    String(String),
    Handle(ObjectHandle),
}

impl ScriptKey {
    /// Try to create a key from a VmSlot.
    /// Returns None for non-hashable types (Native, Void).
    pub fn from_slot(slot: &VmSlot) -> Option<Self> {
        match slot {
            VmSlot::Int(v) => Some(ScriptKey::Int(*v)),
            VmSlot::Float(v) => Some(ScriptKey::Float(OrderedFloat(*v))),
            VmSlot::Bool(v) => Some(ScriptKey::Bool(*v)),
            VmSlot::String(v) => Some(ScriptKey::String(v.clone())),
            VmSlot::Object(h) => Some(ScriptKey::Handle(*h)),
            VmSlot::NullHandle => Some(ScriptKey::Handle(ObjectHandle::null())),
            VmSlot::Void | VmSlot::Native(_) => None,
        }
    }

    /// Convert back to VmSlot
    pub fn to_slot(&self) -> VmSlot {
        match self {
            ScriptKey::Int(v) => VmSlot::Int(*v),
            ScriptKey::Float(v) => VmSlot::Float(v.0),
            ScriptKey::Bool(v) => VmSlot::Bool(*v),
            ScriptKey::String(v) => VmSlot::String(v.clone()),
            ScriptKey::Handle(h) => VmSlot::Object(*h),
        }
    }
}

/// Type-erased dictionary for AngelScript dictionary<K,V> template.
/// Reference type with manual reference counting.
pub struct ScriptDict {
    /// Type-erased storage
    entries: HashMap<ScriptKey, VmSlot>,
    /// Key type for runtime checking
    key_type_id: TypeId,
    /// Value type for runtime checking
    value_type_id: TypeId,
    /// Reference count (starts at 1)
    ref_count: AtomicU32,
}

impl ScriptDict {
    pub fn new(key_type_id: TypeId, value_type_id: TypeId) -> Self {
        Self {
            entries: HashMap::new(),
            key_type_id,
            value_type_id,
            ref_count: AtomicU32::new(1),
        }
    }

    pub fn with_capacity(key_type_id: TypeId, value_type_id: TypeId, capacity: usize) -> Self {
        Self {
            entries: HashMap::with_capacity(capacity),
            key_type_id,
            value_type_id,
            ref_count: AtomicU32::new(1),
        }
    }
}
```

## Methods to Implement (~20)

### Reference Counting
| Method | Signature | Notes |
|--------|-----------|-------|
| `add_ref` | `fn add_ref(&self)` | Increment ref count |
| `release` | `fn release(&self) -> bool` | Decrement, return true if dropped to 0 |
| `ref_count` | `fn ref_count(&self) -> u32` | Get current count |

### Size and Capacity
| Method | Signature | Notes |
|--------|-----------|-------|
| `len` | `fn len(&self) -> u32` | Entry count |
| `is_empty` | `fn is_empty(&self) -> bool` | Zero entries |
| `capacity` | `fn capacity(&self) -> u32` | Allocated capacity |
| `reserve` | `fn reserve(&mut self, additional: u32)` | Reserve space |
| `shrink_to_fit` | `fn shrink_to_fit(&mut self)` | Release excess capacity |

### Insertion and Update
| Method | Signature | Notes |
|--------|-----------|-------|
| `insert` | `fn insert(&mut self, key: ScriptKey, value: VmSlot) -> Option<VmSlot>` | Insert or update, return old value |
| `get_or_insert` | `fn get_or_insert(&mut self, key: ScriptKey, default: VmSlot) -> &VmSlot` | Get existing or insert default |
| `try_insert` | `fn try_insert(&mut self, key: ScriptKey, value: VmSlot) -> bool` | Insert only if absent, return success |

### Retrieval
| Method | Signature | Notes |
|--------|-----------|-------|
| `get` | `fn get(&self, key: &ScriptKey) -> Option<&VmSlot>` | Get value by key |
| `get_mut` | `fn get_mut(&mut self, key: &ScriptKey) -> Option<&mut VmSlot>` | Mutable access |
| `get_or` | `fn get_or(&self, key: &ScriptKey, default: VmSlot) -> VmSlot` | Get or return default |
| `contains_key` | `fn contains_key(&self, key: &ScriptKey) -> bool` | Check key exists |

### Removal
| Method | Signature | Notes |
|--------|-----------|-------|
| `remove` | `fn remove(&mut self, key: &ScriptKey) -> Option<VmSlot>` | Remove and return value |
| `clear` | `fn clear(&mut self)` | Remove all entries |

### Key/Value Access
| Method | Signature | Notes |
|--------|-----------|-------|
| `keys` | `fn keys(&self) -> Vec<ScriptKey>` | All keys as vector |
| `values` | `fn values(&self) -> Vec<VmSlot>` | All values as vector |

### Bulk Operations
| Method | Signature | Notes |
|--------|-----------|-------|
| `extend` | `fn extend(&mut self, other: &Self)` | Insert all from other |
| `retain` | `fn retain<F>(&mut self, f: F)` | Keep matching entries |
| `clone_dict` | `fn clone_dict(&self) -> Self` | Deep clone |

### Predicates
| Method | Signature | Notes |
|--------|-----------|-------|
| `contains_value` | `fn contains_value(&self, value: &VmSlot) -> bool` | Linear search |
| `count_value` | `fn count_value(&self, value: &VmSlot) -> u32` | Count value occurrences |

## Template Validation

When dictionary<K,V> is instantiated, we need to validate that K is hashable:

```rust
/// Check if a TypeId represents a hashable type.
pub fn is_hashable_type(type_id: TypeId) -> bool {
    match type_id {
        // Primitives are hashable
        TypeId::VOID_TYPE => false,
        TypeId::BOOL_TYPE |
        TypeId::INT8_TYPE | TypeId::INT16_TYPE | TypeId::INT32_TYPE | TypeId::INT64_TYPE |
        TypeId::UINT8_TYPE | TypeId::UINT16_TYPE | TypeId::UINT32_TYPE | TypeId::UINT64_TYPE |
        TypeId::FLOAT_TYPE | TypeId::DOUBLE_TYPE |
        TypeId::STRING_TYPE => true,

        // Handles are hashable (by identity)
        id if id.is_handle() => true,

        // Value types are NOT hashable unless they implement opHash
        _ => false,
    }
}
```

## Dependencies

Add to `Cargo.toml`:
```toml
ordered-float = "4.0"
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_and_insert() {
        let mut dict = ScriptDict::new(TypeId::STRING_TYPE, TypeId::INT32);
        dict.insert(
            ScriptKey::String("hello".to_string()),
            VmSlot::Int(42)
        );

        let key = ScriptKey::String("hello".to_string());
        assert_eq!(dict.get(&key), Some(&VmSlot::Int(42)));
    }

    #[test]
    fn test_ref_counting() {
        let dict = ScriptDict::new(TypeId::STRING_TYPE, TypeId::INT32);
        assert_eq!(dict.ref_count(), 1);

        dict.add_ref();
        assert_eq!(dict.ref_count(), 2);

        assert!(!dict.release());
        assert!(dict.release());
    }

    #[test]
    fn test_float_keys() {
        let mut dict = ScriptDict::new(TypeId::DOUBLE_TYPE, TypeId::STRING_TYPE);
        dict.insert(
            ScriptKey::Float(OrderedFloat(3.14)),
            VmSlot::String("pi".to_string())
        );

        let key = ScriptKey::Float(OrderedFloat(3.14));
        assert_eq!(
            dict.get(&key),
            Some(&VmSlot::String("pi".to_string()))
        );
    }

    #[test]
    fn test_keys_and_values() {
        let mut dict = ScriptDict::new(TypeId::INT32, TypeId::INT32);
        dict.insert(ScriptKey::Int(1), VmSlot::Int(10));
        dict.insert(ScriptKey::Int(2), VmSlot::Int(20));

        let keys = dict.keys();
        assert_eq!(keys.len(), 2);

        let values = dict.values();
        assert_eq!(values.len(), 2);
    }
}
```

## Acceptance Criteria

- [ ] `ordered-float` added to Cargo.toml
- [ ] `ScriptKey` enum implemented with Hash + Eq
- [ ] `src/runtime/dict.rs` implements ScriptDict
- [ ] Type-erased storage using `HashMap<ScriptKey, VmSlot>`
- [ ] Reference counting with `AtomicU32`
- [ ] All ~20 methods implemented
- [ ] `is_hashable_type()` helper for template validation
- [ ] Unit tests for all methods
- [ ] `cargo test --lib` passes
