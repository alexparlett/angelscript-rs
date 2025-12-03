# Task 08a: Runtime String Type

**Status:** Not Started
**Parent:** Task 08 - Built-in Modules
**Depends On:** None

---

## Objective

Create `ScriptString` - a Rust backing type that wraps `String` and provides all methods needed for the AngelScript string type.

## Files to Create

- `src/runtime/mod.rs` - Module exports
- `src/runtime/string.rs` - ScriptString implementation

## Design

```rust
/// AngelScript string type backed by Rust String.
/// This is a VALUE type - copied on assignment.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScriptString(String);

impl ScriptString {
    pub fn new() -> Self { ... }
    pub fn from_str(s: &str) -> Self { ... }
    // ... all methods below
}

// Implement standard traits
impl From<String> for ScriptString { ... }
impl From<&str> for ScriptString { ... }
impl Deref for ScriptString { type Target = str; ... }
impl DerefMut for ScriptString { ... }
impl Display for ScriptString { ... }
```

## Methods to Implement (~35)

### Size and Capacity
| Method | Signature | Implementation |
|--------|-----------|----------------|
| `len` | `fn len(&self) -> u32` | `self.0.len() as u32` |
| `is_empty` | `fn is_empty(&self) -> bool` | `self.0.is_empty()` |
| `capacity` | `fn capacity(&self) -> u32` | `self.0.capacity() as u32` |
| `reserve` | `fn reserve(&mut self, additional: u32)` | `self.0.reserve(additional as usize)` |
| `shrink_to_fit` | `fn shrink_to_fit(&mut self)` | `self.0.shrink_to_fit()` |
| `clear` | `fn clear(&mut self)` | `self.0.clear()` |
| `resize` | `fn resize(&mut self, new_len: u32)` | Truncate or pad with null bytes |

### Substring and Slicing
| Method | Signature | Implementation |
|--------|-----------|----------------|
| `substr` | `fn substr(&self, start: u32, count: i32) -> Self` | Handle -1 for "rest of string" |
| `slice` | `fn slice(&self, start: u32, end: u32) -> Self` | `self.0[start..end]` (byte indices) |
| `slice_from` | `fn slice_from(&self, start: u32) -> Self` | `self.0[start..]` |
| `slice_to` | `fn slice_to(&self, end: u32) -> Self` | `self.0[..end]` |

### Search - Substring
| Method | Signature | Implementation |
|--------|-----------|----------------|
| `find_first` | `fn find_first(&self, s: &Self, start: u32) -> i32` | `str::find` from position, -1 if not found |
| `find_last` | `fn find_last(&self, s: &Self, start: i32) -> i32` | `str::rfind`, -1 means from end |

### Search - Character Sets
| Method | Signature | Implementation |
|--------|-----------|----------------|
| `find_first_of` | `fn find_first_of(&self, chars: &Self, start: u32) -> i32` | Find first char in set |
| `find_first_not_of` | `fn find_first_not_of(&self, chars: &Self, start: u32) -> i32` | Find first char not in set |
| `find_last_of` | `fn find_last_of(&self, chars: &Self, start: i32) -> i32` | From end |
| `find_last_not_of` | `fn find_last_not_of(&self, chars: &Self, start: i32) -> i32` | From end |

### Modification
| Method | Signature | Implementation |
|--------|-----------|----------------|
| `insert` | `fn insert(&mut self, pos: u32, s: &Self)` | `String::insert_str` |
| `erase` | `fn erase(&mut self, pos: u32, count: i32)` | `String::drain`, -1 means rest |
| `push` | `fn push(&mut self, c: u8)` | Push single byte |
| `pop` | `fn pop(&mut self) -> u8` | Pop and return last byte |
| `truncate` | `fn truncate(&mut self, len: u32)` | `String::truncate` |
| `replace_range` | `fn replace_range(&mut self, start: u32, end: u32, s: &Self)` | `String::replace_range` |

### Case Conversion
| Method | Signature | Implementation |
|--------|-----------|----------------|
| `to_lowercase` | `fn to_lowercase(&self) -> Self` | `str::to_lowercase` |
| `to_uppercase` | `fn to_uppercase(&self) -> Self` | `str::to_uppercase` |
| `to_ascii_lowercase` | `fn to_ascii_lowercase(&self) -> Self` | `str::to_ascii_lowercase` |
| `to_ascii_uppercase` | `fn to_ascii_uppercase(&self) -> Self` | `str::to_ascii_uppercase` |

### Trimming
| Method | Signature | Implementation |
|--------|-----------|----------------|
| `trim` | `fn trim(&self) -> Self` | `str::trim` |
| `trim_start` | `fn trim_start(&self) -> Self` | `str::trim_start` |
| `trim_end` | `fn trim_end(&self) -> Self` | `str::trim_end` |
| `trim_matches` | `fn trim_matches(&self, chars: &Self) -> Self` | Trim matching chars |

### Predicates
| Method | Signature | Implementation |
|--------|-----------|----------------|
| `starts_with` | `fn starts_with(&self, s: &Self) -> bool` | `str::starts_with` |
| `ends_with` | `fn ends_with(&self, s: &Self) -> bool` | `str::ends_with` |
| `contains` | `fn contains(&self, s: &Self) -> bool` | `str::contains` |
| `is_ascii` | `fn is_ascii(&self) -> bool` | `str::is_ascii` |
| `is_ascii_alphabetic` | `fn is_ascii_alphabetic(&self) -> bool` | All chars alphabetic |
| `is_ascii_alphanumeric` | `fn is_ascii_alphanumeric(&self) -> bool` | All chars alphanumeric |
| `is_ascii_digit` | `fn is_ascii_digit(&self) -> bool` | All chars digits |
| `is_ascii_hexdigit` | `fn is_ascii_hexdigit(&self) -> bool` | All chars hex digits |
| `is_ascii_whitespace` | `fn is_ascii_whitespace(&self) -> bool` | All chars whitespace |

### Transform
| Method | Signature | Implementation |
|--------|-----------|----------------|
| `repeat` | `fn repeat(&self, count: u32) -> Self` | `str::repeat` |
| `replace_all` | `fn replace_all(&self, from: &Self, to: &Self) -> Self` | `str::replace` |
| `replace_first` | `fn replace_first(&self, from: &Self, to: &Self) -> Self` | `str::replacen(..., 1)` |
| `reversed` | `fn reversed(&self) -> Self` | `chars().rev().collect()` |
| `count_occurrences` | `fn count_occurrences(&self, s: &Self) -> u32` | `str::matches().count()` |

### Operators (for ClassBuilder registration)
| Method | Signature | Implementation |
|--------|-----------|----------------|
| `assign` | `fn assign(&mut self, other: &Self) -> &mut Self` | Clone from other |
| `concat` | `fn concat(&self, other: &Self) -> Self` | `self + other` |
| `concat_r` | `fn concat_r(&self, other: &Self) -> Self` | `other + self` |
| `push_str` | `fn push_str(&mut self, other: &Self) -> &mut Self` | `+=` operator |
| `eq` | `fn eq(&self, other: &Self) -> bool` | `==` operator |
| `cmp` | `fn cmp(&self, other: &Self) -> i32` | `<0, 0, >0` comparison |
| `byte_at` | `fn byte_at(&self, index: u32) -> u8` | Index access |
| `byte_at_mut` | `fn byte_at_mut(&mut self, index: u32) -> &mut u8` | Mutable index access |

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_and_from() {
        let s = ScriptString::new();
        assert!(s.is_empty());

        let s = ScriptString::from("hello");
        assert_eq!(s.len(), 5);
    }

    #[test]
    fn test_substr() {
        let s = ScriptString::from("hello world");
        assert_eq!(s.substr(0, 5).as_str(), "hello");
        assert_eq!(s.substr(6, -1).as_str(), "world");
    }

    #[test]
    fn test_find_first() {
        let s = ScriptString::from("hello hello");
        assert_eq!(s.find_first(&"ell".into(), 0), 1);
        assert_eq!(s.find_first(&"ell".into(), 2), 7);
        assert_eq!(s.find_first(&"xyz".into(), 0), -1);
    }

    #[test]
    fn test_case_conversion() {
        let s = ScriptString::from("Hello World");
        assert_eq!(s.to_lowercase().as_str(), "hello world");
        assert_eq!(s.to_uppercase().as_str(), "HELLO WORLD");
    }

    // ... more tests for each method
}
```

## Acceptance Criteria

- [ ] `src/runtime/mod.rs` created with `pub use string::ScriptString;`
- [ ] `src/runtime/string.rs` implements ScriptString
- [ ] All ~35 methods implemented
- [ ] Standard traits: `Clone`, `Default`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`, `Display`, `Deref`, `DerefMut`
- [ ] Conversions: `From<String>`, `From<&str>`, `Into<String>`
- [ ] Unit tests for all methods
- [ ] `cargo test --lib` passes
