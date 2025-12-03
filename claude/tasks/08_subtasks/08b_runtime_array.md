# Task 08b: Runtime Array Type

**Status:** Not Started
**Parent:** Task 08 - Built-in Modules
**Depends On:** 08a (runtime module exists)

---

## Objective

Create `ScriptArray` - a type-erased, reference-counted array type that backs the AngelScript `array<T>` template.

## Files to Modify

- `src/runtime/mod.rs` - Add export
- `src/runtime/array.rs` - ScriptArray implementation (new)

## Design Considerations

### Why Type-Erased?

The VM operates on `VmSlot` values. When script code does `array<int> a;`, the runtime doesn't have a Rust generic `Vec<i32>` - it has a `Vec<VmSlot>` where each slot contains an `Int(i64)`. This makes the array homogeneous at runtime but heterogeneous at the Rust level.

### Reference Counting

Arrays are REFERENCE types in AngelScript - they're heap-allocated and passed by handle. Multiple handles can point to the same array. We use `AtomicU32` for thread-safe reference counting.

## Implementation

```rust
use std::sync::atomic::{AtomicU32, Ordering};

/// Type-erased array for AngelScript array<T> template.
/// Reference type with manual reference counting.
pub struct ScriptArray {
    /// Type-erased element storage
    elements: Vec<VmSlot>,
    /// Element type for runtime checking
    element_type_id: TypeId,
    /// Reference count (starts at 1)
    ref_count: AtomicU32,
}

impl ScriptArray {
    /// Create empty array for given element type
    pub fn new(element_type_id: TypeId) -> Self {
        Self {
            elements: Vec::new(),
            element_type_id,
            ref_count: AtomicU32::new(1),
        }
    }

    /// Create array with initial capacity
    pub fn with_capacity(element_type_id: TypeId, capacity: usize) -> Self {
        Self {
            elements: Vec::with_capacity(capacity),
            element_type_id,
            ref_count: AtomicU32::new(1),
        }
    }

    /// Create array filled with default values
    pub fn with_length(element_type_id: TypeId, length: usize) -> Self {
        let default = VmSlot::default_for_type(element_type_id);
        Self {
            elements: vec![default; length],
            element_type_id,
            ref_count: AtomicU32::new(1),
        }
    }

    /// Create array filled with specific value
    pub fn filled(element_type_id: TypeId, length: usize, value: VmSlot) -> Self {
        Self {
            elements: vec![value; length],
            element_type_id,
            ref_count: AtomicU32::new(1),
        }
    }
}
```

## Methods to Implement (~35)

### Reference Counting
| Method | Signature | Notes |
|--------|-----------|-------|
| `add_ref` | `fn add_ref(&self)` | Increment ref count |
| `release` | `fn release(&self) -> bool` | Decrement, return true if dropped to 0 |
| `ref_count` | `fn ref_count(&self) -> u32` | Get current count |

### Size and Capacity
| Method | Signature | Notes |
|--------|-----------|-------|
| `len` | `fn len(&self) -> u32` | Element count |
| `is_empty` | `fn is_empty(&self) -> bool` | Zero elements |
| `capacity` | `fn capacity(&self) -> u32` | Allocated capacity |
| `reserve` | `fn reserve(&mut self, additional: u32)` | Reserve space |
| `shrink_to_fit` | `fn shrink_to_fit(&mut self)` | Release excess capacity |
| `clear` | `fn clear(&mut self)` | Remove all elements |
| `resize_with_default` | `fn resize_with_default(&mut self, new_len: u32)` | Resize with type default |

### Element Access
| Method | Signature | Notes |
|--------|-----------|-------|
| `get` | `fn get(&self, index: u32) -> Option<&VmSlot>` | Bounds-checked access |
| `get_mut` | `fn get_mut(&mut self, index: u32) -> Option<&mut VmSlot>` | Mutable access |
| `first` | `fn first(&self) -> Option<&VmSlot>` | First element |
| `first_mut` | `fn first_mut(&mut self) -> Option<&mut VmSlot>` | Mutable first |
| `last` | `fn last(&self) -> Option<&VmSlot>` | Last element |
| `last_mut` | `fn last_mut(&mut self) -> Option<&mut VmSlot>` | Mutable last |

### Insertion
| Method | Signature | Notes |
|--------|-----------|-------|
| `push` | `fn push(&mut self, value: VmSlot)` | Append element |
| `insert` | `fn insert(&mut self, index: u32, value: VmSlot)` | Insert at position |
| `extend` | `fn extend(&mut self, other: &Self)` | Append all from other |

### Removal
| Method | Signature | Notes |
|--------|-----------|-------|
| `pop` | `fn pop(&mut self) -> Option<VmSlot>` | Remove and return last |
| `remove_at` | `fn remove_at(&mut self, index: u32) -> Option<VmSlot>` | Remove at position |
| `remove_range` | `fn remove_range(&mut self, start: u32, count: u32)` | Remove range |
| `retain` | `fn retain<F>(&mut self, f: F)` | Keep matching elements |
| `dedup` | `fn dedup(&mut self)` | Remove consecutive duplicates |

### Search
| Method | Signature | Notes |
|--------|-----------|-------|
| `find` | `fn find(&self, value: &VmSlot) -> Option<u32>` | First occurrence |
| `find_from` | `fn find_from(&self, start: u32, value: &VmSlot) -> Option<u32>` | From position |
| `rfind` | `fn rfind(&self, value: &VmSlot) -> Option<u32>` | Last occurrence |
| `contains` | `fn contains(&self, value: &VmSlot) -> bool` | Element exists |
| `count` | `fn count(&self, value: &VmSlot) -> u32` | Count occurrences |

### Ordering
| Method | Signature | Notes |
|--------|-----------|-------|
| `reverse` | `fn reverse(&mut self)` | Reverse in place |
| `sort_ascending` | `fn sort_ascending(&mut self)` | Sort ascending |
| `sort_descending` | `fn sort_descending(&mut self)` | Sort descending |
| `is_sorted` | `fn is_sorted(&self) -> bool` | Check if sorted ascending |
| `is_sorted_desc` | `fn is_sorted_desc(&self) -> bool` | Check if sorted descending |

### Transform
| Method | Signature | Notes |
|--------|-----------|-------|
| `fill` | `fn fill(&mut self, value: VmSlot)` | Set all elements |
| `swap` | `fn swap(&mut self, i: u32, j: u32)` | Swap two elements |
| `rotate` | `fn rotate(&mut self, amount: i32)` | Rotate left (neg) or right (pos) |

### Slicing and Cloning
| Method | Signature | Notes |
|--------|-----------|-------|
| `slice` | `fn slice(&self, start: u32, end: u32) -> Self` | Create new array from range |
| `slice_from` | `fn slice_from(&self, start: u32) -> Self` | From position to end |
| `slice_to` | `fn slice_to(&self, end: u32) -> Self` | From start to position |
| `clone_array` | `fn clone_array(&self) -> Self` | Deep clone |

### Binary Search (for sorted arrays)
| Method | Signature | Notes |
|--------|-----------|-------|
| `binary_search` | `fn binary_search(&self, value: &VmSlot) -> Result<u32, u32>` | Returns Ok(index) or Err(insert_pos) |

## VmSlot Comparison

For search and sort operations, we need `VmSlot` to support comparison:

```rust
impl VmSlot {
    /// Compare two slots for equality (type-aware)
    pub fn eq_slot(&self, other: &Self) -> bool {
        match (self, other) {
            (VmSlot::Int(a), VmSlot::Int(b)) => a == b,
            (VmSlot::Float(a), VmSlot::Float(b)) => a == b,
            (VmSlot::Bool(a), VmSlot::Bool(b)) => a == b,
            (VmSlot::String(a), VmSlot::String(b)) => a == b,
            (VmSlot::Object(a), VmSlot::Object(b)) => a == b,  // Handle comparison
            (VmSlot::NullHandle, VmSlot::NullHandle) => true,
            _ => false,
        }
    }

    /// Compare two slots for ordering (type-aware)
    pub fn cmp_slot(&self, other: &Self) -> std::cmp::Ordering {
        // Implementation for sorting
    }

    /// Get default value for a type
    pub fn default_for_type(type_id: TypeId) -> Self {
        // Return appropriate default based on type
    }
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_and_push() {
        let mut arr = ScriptArray::new(TypeId::INT32);
        arr.push(VmSlot::Int(1));
        arr.push(VmSlot::Int(2));
        arr.push(VmSlot::Int(3));
        assert_eq!(arr.len(), 3);
    }

    #[test]
    fn test_ref_counting() {
        let arr = ScriptArray::new(TypeId::INT32);
        assert_eq!(arr.ref_count(), 1);

        arr.add_ref();
        assert_eq!(arr.ref_count(), 2);

        assert!(!arr.release()); // Not zero yet
        assert_eq!(arr.ref_count(), 1);

        assert!(arr.release()); // Now zero
    }

    #[test]
    fn test_find() {
        let mut arr = ScriptArray::new(TypeId::INT32);
        arr.push(VmSlot::Int(10));
        arr.push(VmSlot::Int(20));
        arr.push(VmSlot::Int(30));

        assert_eq!(arr.find(&VmSlot::Int(20)), Some(1));
        assert_eq!(arr.find(&VmSlot::Int(99)), None);
    }

    #[test]
    fn test_sort() {
        let mut arr = ScriptArray::new(TypeId::INT32);
        arr.push(VmSlot::Int(3));
        arr.push(VmSlot::Int(1));
        arr.push(VmSlot::Int(2));

        arr.sort_ascending();
        assert_eq!(arr.get(0), Some(&VmSlot::Int(1)));
        assert_eq!(arr.get(1), Some(&VmSlot::Int(2)));
        assert_eq!(arr.get(2), Some(&VmSlot::Int(3)));
    }
}
```

## Acceptance Criteria

- [ ] `src/runtime/array.rs` implements ScriptArray
- [ ] Type-erased storage using `Vec<VmSlot>`
- [ ] Reference counting with `AtomicU32`
- [ ] All ~35 methods implemented
- [ ] VmSlot comparison methods for search/sort
- [ ] Unit tests for all methods
- [ ] `cargo test --lib` passes
