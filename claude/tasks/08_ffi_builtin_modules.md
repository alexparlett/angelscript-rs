# Task 08: Built-in Modules

**Status:** Not Started
**Depends On:** Tasks 01-05, 07
**Estimated Scope:** Implement standard library modules using FFI registration API

---

## Objective

Implement the built-in modules (std, string, array, dictionary, math) using the FFI registration API with declaration string parsing. This replaces the hardcoded implementations in registry.rs.

## Files to Create

- `src/modules/mod.rs` - Module exports and default_modules()
- `src/modules/std.rs` - print, println, eprint, eprintln
- `src/modules/string.rs` - String type with methods
- `src/modules/array.rs` - array<T> template
- `src/modules/dictionary.rs` - dictionary<K,V> template
- `src/modules/math.rs` - Math functions and constants

## Current Hardcoded Implementation

Located in `src/semantic/types/registry.rs` (~3500 lines), including:
- String type with ~20 methods (length, substr, findFirst, etc.)
- String operators (+, ==, !=, <, >, etc.)
- Array template with methods (length, resize, insertLast, etc.)
- Dictionary template

## New Implementation

### mod.rs

```rust
// src/modules/mod.rs

mod std;
mod string;
mod array;
mod dictionary;
mod math;

pub use self::std::std;
pub use self::string::string;
pub use self::array::array;
pub use self::dictionary::dictionary;
pub use self::math::math;

use crate::ffi::{Module, FfiRegistrationError};

/// Returns all default modules
pub fn default_modules() -> Result<Vec<Module<'static>>, FfiRegistrationError> {
    Ok(vec![
        std()?,
        string()?,
        array()?,
        dictionary()?,
        math()?,
    ])
}
```

### std.rs

```rust
// src/modules/std.rs

use crate::ffi::{Module, FfiRegistrationError};

pub fn std() -> Result<Module<'static>, FfiRegistrationError> {
    let mut module = Module::root();

    // Print without newline
    module.register_fn("void print(const string &in s)", |s: &str| {
        print!("{}", s);
    })?;

    // Print with newline
    module.register_fn("void println(const string &in s)", |s: &str| {
        println!("{}", s);
    })?;

    // Print to stderr without newline
    module.register_fn("void eprint(const string &in s)", |s: &str| {
        eprint!("{}", s);
    })?;

    // Print to stderr with newline
    module.register_fn("void eprintln(const string &in s)", |s: &str| {
        eprintln!("{}", s);
    })?;

    Ok(module)
}
```

### string.rs

```rust
// src/modules/string.rs
//
// String type backed by Rust's String. Provides AngelScript standard library
// compatibility plus additional Rust String methods.

use crate::ffi::{Module, FfiRegistrationError};
use crate::runtime::ScriptString;

pub fn string() -> Result<Module<'static>, FfiRegistrationError> {
    let mut module = Module::root();

    // =========================================================================
    // STRING TYPE REGISTRATION
    // =========================================================================

    module.register_type::<ScriptString>("string")
        .value_type()

        // =====================================================================
        // CONSTRUCTORS
        // =====================================================================

        .constructor("void f()", ScriptString::new)?
        .constructor("void f(const string &in)", ScriptString::clone)?
        .constructor("void f(uint8 c)", |c: u8| ScriptString::from(char::from(c)))?
        .constructor("void f(uint8 c, uint count)", |c: u8, count: u32| {
            ScriptString::from(String::from(char::from(c)).repeat(count as usize))
        })?

        // =====================================================================
        // OPERATORS (AngelScript standard)
        // =====================================================================

        // Assignment
        .operator("string& opAssign(const string &in)", ScriptString::assign)?

        // Concatenation
        .operator("string opAdd(const string &in)", ScriptString::concat)?
        .operator("string opAdd_r(const string &in)", ScriptString::concat_r)?
        .operator("string& opAddAssign(const string &in)", ScriptString::push_str)?

        // Comparison
        .operator("bool opEquals(const string &in)", ScriptString::eq)?
        .operator("int opCmp(const string &in)", ScriptString::cmp)?

        // Index access (byte-based, as per AngelScript spec)
        .operator("uint8 opIndex(uint)", ScriptString::byte_at)?
        .operator("uint8& opIndex(uint)", ScriptString::byte_at_mut)?

        // =====================================================================
        // SIZE AND CAPACITY (AngelScript standard + Rust extras)
        // =====================================================================

        // AngelScript standard
        .method("uint length() const", ScriptString::len)?                      // String::len
        .method("bool isEmpty() const", ScriptString::is_empty)?                // String::is_empty
        .method("void resize(uint)", ScriptString::resize)?                     // Resize to length

        // Rust extras
        .method("uint capacity() const", ScriptString::capacity)?               // String::capacity
        .method("void reserve(uint)", ScriptString::reserve)?                   // String::reserve
        .method("void shrinkToFit()", ScriptString::shrink_to_fit)?             // String::shrink_to_fit
        .method("void clear()", ScriptString::clear)?                           // String::clear

        // =====================================================================
        // SUBSTRING AND SLICING (AngelScript standard + Rust extras)
        // =====================================================================

        // AngelScript standard
        .method("string substr(uint start = 0, int count = -1) const", ScriptString::substr)?

        // Rust extras
        .method("string slice(uint start, uint end) const", ScriptString::slice)?           // Byte slice [start..end]
        .method("string sliceFrom(uint start) const", ScriptString::slice_from)?            // [start..]
        .method("string sliceTo(uint end) const", ScriptString::slice_to)?                  // [..end]

        // =====================================================================
        // SEARCH - FIND SUBSTRING (AngelScript standard)
        // =====================================================================

        .method("int findFirst(const string &in, uint start = 0) const", ScriptString::find_first)?
        .method("int findLast(const string &in, int start = -1) const", ScriptString::find_last)?

        // =====================================================================
        // SEARCH - FIND CHARACTERS (AngelScript standard)
        // =====================================================================

        .method("int findFirstOf(const string &in chars, uint start = 0) const", ScriptString::find_first_of)?
        .method("int findFirstNotOf(const string &in chars, uint start = 0) const", ScriptString::find_first_not_of)?
        .method("int findLastOf(const string &in chars, int start = -1) const", ScriptString::find_last_of)?
        .method("int findLastNotOf(const string &in chars, int start = -1) const", ScriptString::find_last_not_of)?

        // =====================================================================
        // MODIFICATION (AngelScript standard + Rust extras)
        // =====================================================================

        // AngelScript standard
        .method("void insert(uint pos, const string &in)", ScriptString::insert)?
        .method("void erase(uint pos, int count = -1)", ScriptString::erase)?

        // Rust extras
        .method("void push(uint8 c)", ScriptString::push)?                      // Push single byte
        .method("uint8 pop()", ScriptString::pop)?                              // Pop and return last byte
        .method("void truncate(uint len)", ScriptString::truncate)?             // String::truncate
        .method("void replace(uint start, uint end, const string &in)", ScriptString::replace_range)?

        // =====================================================================
        // CASE CONVERSION (Rust String methods)
        // =====================================================================

        .method("string toLower() const", ScriptString::to_lowercase)?          // String::to_lowercase
        .method("string toUpper() const", ScriptString::to_uppercase)?          // String::to_uppercase
        .method("string toAsciiLower() const", ScriptString::to_ascii_lowercase)?   // make_ascii_lowercase
        .method("string toAsciiUpper() const", ScriptString::to_ascii_uppercase)?   // make_ascii_uppercase

        // =====================================================================
        // TRIMMING (Rust String methods)
        // =====================================================================

        .method("string trim() const", ScriptString::trim)?                     // str::trim
        .method("string trimStart() const", ScriptString::trim_start)?          // str::trim_start
        .method("string trimEnd() const", ScriptString::trim_end)?              // str::trim_end
        .method("string trimMatches(const string &in chars) const", ScriptString::trim_matches)?

        // =====================================================================
        // PREDICATES (Rust String methods)
        // =====================================================================

        .method("bool startsWith(const string &in) const", ScriptString::starts_with)?   // str::starts_with
        .method("bool endsWith(const string &in) const", ScriptString::ends_with)?       // str::ends_with
        .method("bool contains(const string &in) const", ScriptString::contains)?        // str::contains

        // Character class predicates
        .method("bool isAscii() const", ScriptString::is_ascii)?                // str::is_ascii
        .method("bool isAsciiAlphabetic() const", ScriptString::is_ascii_alphabetic)?
        .method("bool isAsciiAlphanumeric() const", ScriptString::is_ascii_alphanumeric)?
        .method("bool isAsciiDigit() const", ScriptString::is_ascii_digit)?
        .method("bool isAsciiHexdigit() const", ScriptString::is_ascii_hexdigit)?
        .method("bool isAsciiWhitespace() const", ScriptString::is_ascii_whitespace)?

        // =====================================================================
        // SPLIT AND JOIN (AngelScript standard)
        // =====================================================================

        .method("array<string>@ split(const string &in delimiter) const", ScriptString::split)?

        // =====================================================================
        // REPEAT AND REPLACE (Rust String methods)
        // =====================================================================

        .method("string repeat(uint count) const", ScriptString::repeat)?       // str::repeat
        .method("string replaceAll(const string &in from, const string &in to) const", ScriptString::replace_all)?
        .method("string replaceFirst(const string &in from, const string &in to) const", ScriptString::replace_first)?

        // =====================================================================
        // LINES AND CHARS (Rust iterators as arrays)
        // =====================================================================

        .method("array<string>@ lines() const", ScriptString::lines)?           // str::lines
        .method("array<string>@ splitWhitespace() const", ScriptString::split_whitespace)?
        .method("array<uint>@ chars() const", ScriptString::chars)?             // Unicode codepoints
        .method("array<uint8>@ bytes() const", ScriptString::bytes)?            // Raw bytes

        // =====================================================================
        // UTILITY
        // =====================================================================

        .method("uint countOccurrences(const string &in) const", ScriptString::count_occurrences)?  // str::matches().count()
        .method("string reversed() const", ScriptString::reversed)?             // chars().rev().collect()

        .build()?;

    // =========================================================================
    // GLOBAL STRING FUNCTIONS (AngelScript standard)
    // =========================================================================

    // Join array of strings
    module.register_fn("string join(const array<string> &in, const string &in delimiter)",
        string_join)?;

    // Parsing functions
    module.register_fn("int64 parseInt(const string &in, uint base = 10, uint &out byteCount = 0)",
        string_parse_int)?;
    module.register_fn("uint64 parseUInt(const string &in, uint base = 10, uint &out byteCount = 0)",
        string_parse_uint)?;
    module.register_fn("double parseFloat(const string &in, uint &out byteCount = 0)",
        string_parse_float)?;

    // Formatting functions
    module.register_fn("string formatInt(int64 val, const string &in options = \"\", uint width = 0)",
        string_format_int)?;
    module.register_fn("string formatUInt(uint64 val, const string &in options = \"\", uint width = 0)",
        string_format_uint)?;
    module.register_fn("string formatFloat(double val, const string &in options = \"\", uint width = 0, uint precision = 6)",
        string_format_float)?;

    Ok(module)
}

// Helper function implementations would go here...
```

### String Module Function Summary

| Category | Methods | Rust Source |
|----------|---------|-------------|
| **Size/Capacity** | length, isEmpty, resize, capacity, reserve, shrinkToFit, clear | `String::len`, `is_empty`, `capacity`, `reserve`, `shrink_to_fit`, `clear` |
| **Substring** | substr, slice, sliceFrom, sliceTo | `&str[..]` slicing |
| **Search (substring)** | findFirst, findLast | `str::find`, `str::rfind` |
| **Search (chars)** | findFirstOf, findFirstNotOf, findLastOf, findLastNotOf | `str::find(char::is_...)` |
| **Modification** | insert, erase, push, pop, truncate, replaceRange | `String::insert_str`, `String::drain`, `push`, `pop`, `truncate`, `replace_range` |
| **Case Conversion** | toLower, toUpper, toAsciiLower, toAsciiUpper | `to_lowercase`, `to_uppercase`, `make_ascii_lowercase/uppercase` |
| **Trimming** | trim, trimStart, trimEnd, trimMatches | `str::trim`, `trim_start`, `trim_end`, `trim_matches` |
| **Predicates** | startsWith, endsWith, contains, isAscii, isAsciiAlphabetic, etc. | `starts_with`, `ends_with`, `contains`, `is_ascii`, etc. |
| **Split/Join** | split, lines, splitWhitespace | `str::split`, `lines`, `split_whitespace` |
| **Transform** | repeat, replaceAll, replaceFirst, reversed | `str::repeat`, `str::replace`, `str::replacen`, `chars().rev()` |
| **Iteration** | chars, bytes | `str::chars`, `str::bytes` |
| **Global Functions** | join, parseInt, parseUInt, parseFloat, formatInt, formatUInt, formatFloat | Parsing/formatting utilities |

### array.rs

```rust
// src/modules/array.rs
//
// Dynamic array template backed by Rust's Vec<T>. Provides AngelScript standard
// library compatibility plus additional Rust Vec methods.

use crate::ffi::{Module, FfiRegistrationError, TemplateValidation, CallContext, NativeError};
use crate::runtime::ScriptArray;

pub fn array() -> Result<Module<'static>, FfiRegistrationError> {
    let mut module = Module::root();

    // =========================================================================
    // ARRAY<T> TYPE REGISTRATION
    // =========================================================================

    module.register_type::<ScriptArray>("array<class T>")
        .reference_type()
        .template_callback(|_| TemplateValidation::valid())?

        // =====================================================================
        // FACTORIES (AngelScript standard)
        // =====================================================================

        .factory("array<T>@ f()", ScriptArray::new)?
        .factory("array<T>@ f(uint length)", ScriptArray::with_length)?
        .factory("array<T>@ f(uint length, const T &in value)", ScriptArray::filled)?

        // Reference counting
        .addref(ScriptArray::add_ref)
        .release(ScriptArray::release)

        // =====================================================================
        // OPERATORS (AngelScript standard)
        // =====================================================================

        // Assignment
        .operator("array<T>& opAssign(const array<T> &in)", array_assign)?

        // Equality
        .operator("bool opEquals(const array<T> &in) const", array_equals)?

        // Index access
        .operator("T& opIndex(uint)", array_index)?
        .operator("const T& opIndex(uint) const", array_index_const)?

        // =====================================================================
        // SIZE AND CAPACITY (AngelScript standard + Rust extras)
        // =====================================================================

        // AngelScript standard
        .method("uint length() const", ScriptArray::len)?                       // Vec::len
        .method("bool isEmpty() const", ScriptArray::is_empty)?                 // Vec::is_empty
        .method("void resize(uint length)", array_resize)?                      // Vec::resize
        .method("void reserve(uint length)", ScriptArray::reserve)?             // Vec::reserve

        // Rust extras
        .method("uint capacity() const", ScriptArray::capacity)?                // Vec::capacity
        .method("void shrinkToFit()", ScriptArray::shrink_to_fit)?              // Vec::shrink_to_fit
        .method("void clear()", ScriptArray::clear)?                            // Vec::clear

        // =====================================================================
        // ELEMENT ACCESS (Rust extras)
        // =====================================================================

        .method("T& first()", array_first)?                                     // Vec::first
        .method("const T& first() const", array_first_const)?
        .method("T& last()", array_last)?                                       // Vec::last
        .method("const T& last() const", array_last_const)?

        // =====================================================================
        // INSERTION (AngelScript standard + Rust extras)
        // =====================================================================

        // AngelScript standard
        .method("void insertAt(uint index, const T &in value)", array_insert_at)?
        .method("void insertAt(uint index, const array<T> &in arr)", array_insert_array_at)?
        .method("void insertLast(const T &in value)", array_insert_last)?       // Vec::push

        // Rust extras
        .method("void insertFirst(const T &in value)", array_insert_first)?     // Insert at 0
        .method("void extend(const array<T> &in arr)", array_extend)?           // Vec::extend

        // =====================================================================
        // REMOVAL (AngelScript standard + Rust extras)
        // =====================================================================

        // AngelScript standard
        .method("void removeAt(uint index)", ScriptArray::remove_at)?           // Vec::remove
        .method("void removeLast()", ScriptArray::pop)?                         // Vec::pop
        .method("void removeRange(uint start, uint count)", array_remove_range)? // Vec::drain

        // Rust extras
        .method("void removeFirst()", array_remove_first)?                      // Remove at 0
        .method("T popLast()", array_pop_last)?                                 // Vec::pop with return
        .method("T popFirst()", array_pop_first)?                               // Remove and return first
        .method("void retain(const T &in value)", array_retain)?                // Keep only matching
        .method("void dedup()", array_dedup)?                                   // Vec::dedup (remove consecutive duplicates)

        // =====================================================================
        // SEARCH (AngelScript standard + Rust extras)
        // =====================================================================

        // AngelScript standard
        .method("int find(const T &in value) const", array_find)?
        .method("int find(uint startAt, const T &in value) const", array_find_from)?
        .method("int findByRef(const T &in value) const", array_find_by_ref)?
        .method("int findByRef(uint startAt, const T &in value) const", array_find_by_ref_from)?

        // Rust extras
        .method("int rfind(const T &in value) const", array_rfind)?             // Search from end
        .method("bool contains(const T &in value) const", array_contains)?      // Vec::contains
        .method("uint count(const T &in value) const", array_count)?            // Count occurrences

        // =====================================================================
        // ORDERING (AngelScript standard + Rust extras)
        // =====================================================================

        // AngelScript standard
        .method("void reverse()", ScriptArray::reverse)?                        // Vec::reverse
        .method("void sortAsc()", array_sort_asc)?
        .method("void sortAsc(uint startAt, uint count)", array_sort_asc_range)?
        .method("void sortDesc()", array_sort_desc)?
        .method("void sortDesc(uint startAt, uint count)", array_sort_desc_range)?

        // Rust extras
        .method("bool isSorted() const", array_is_sorted)?                      // is_sorted
        .method("bool isSortedDesc() const", array_is_sorted_desc)?
        .method("void shuffle()", array_shuffle)?                               // Requires rand - consider

        // =====================================================================
        // TRANSFORMATION (Rust extras)
        // =====================================================================

        .method("void fill(const T &in value)", array_fill)?                    // Vec::fill
        .method("void swap(uint i, uint j)", array_swap)?                       // Vec::swap
        .method("void rotate(int amount)", array_rotate)?                       // Vec::rotate_left/right

        // =====================================================================
        // SLICING (Rust extras)
        // =====================================================================

        .method("array<T>@ slice(uint start, uint end) const", array_slice)?    // &[start..end]
        .method("array<T>@ sliceFrom(uint start) const", array_slice_from)?     // &[start..]
        .method("array<T>@ sliceTo(uint end) const", array_slice_to)?           // &[..end]
        .method("array<T>@ clone() const", array_clone)?                        // Clone array

        // =====================================================================
        // BINARY SEARCH (Rust extras - for sorted arrays)
        // =====================================================================

        .method("int binarySearch(const T &in value) const", array_binary_search)?  // binary_search

        .build()?;

    Ok(module)
}

// =============================================================================
// HELPER FUNCTION IMPLEMENTATIONS
// =============================================================================

fn array_assign(ctx: &mut CallContext) -> Result<(), NativeError> {
    let other = ctx.arg_array(0)?;
    let this = ctx.this_mut::<ScriptArray>()?;
    this.clone_from(other);
    ctx.set_return_ref(this)?;
    Ok(())
}

fn array_equals(ctx: &mut CallContext) -> Result<(), NativeError> {
    let other = ctx.arg_array(0)?;
    let this = ctx.this_ref::<ScriptArray>()?;
    ctx.set_return(this.eq(other))?;
    Ok(())
}

fn array_index(ctx: &mut CallContext) -> Result<(), NativeError> {
    let index: u32 = ctx.arg(0)?;
    let this = ctx.this_mut::<ScriptArray>()?;
    ctx.set_return_ref(this.get_mut(index as usize)?)?;
    Ok(())
}

fn array_index_const(ctx: &mut CallContext) -> Result<(), NativeError> {
    let index: u32 = ctx.arg(0)?;
    let this = ctx.this_ref::<ScriptArray>()?;
    ctx.set_return_ref(this.get(index as usize)?)?;
    Ok(())
}

fn array_resize(ctx: &mut CallContext) -> Result<(), NativeError> {
    let length: u32 = ctx.arg(0)?;
    let this = ctx.this_mut::<ScriptArray>()?;
    this.resize_with_default(length as usize)?;
    Ok(())
}

fn array_insert_at(ctx: &mut CallContext) -> Result<(), NativeError> {
    let index: u32 = ctx.arg(0)?;
    let value = ctx.arg_any(1)?;
    let this = ctx.this_mut::<ScriptArray>()?;
    this.insert(index as usize, value)?;
    Ok(())
}

fn array_insert_last(ctx: &mut CallContext) -> Result<(), NativeError> {
    let value = ctx.arg_any(0)?;
    let this = ctx.this_mut::<ScriptArray>()?;
    this.push(value);
    Ok(())
}

fn array_find(ctx: &mut CallContext) -> Result<(), NativeError> {
    let value = ctx.arg_any(0)?;
    let this = ctx.this_ref::<ScriptArray>()?;
    let result = this.find(&value).map(|i| i as i32).unwrap_or(-1);
    ctx.set_return(result)?;
    Ok(())
}

fn array_find_from(ctx: &mut CallContext) -> Result<(), NativeError> {
    let start: u32 = ctx.arg(0)?;
    let value = ctx.arg_any(1)?;
    let this = ctx.this_ref::<ScriptArray>()?;
    let result = this.find_from(start as usize, &value).map(|i| i as i32).unwrap_or(-1);
    ctx.set_return(result)?;
    Ok(())
}

fn array_sort_asc(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_mut::<ScriptArray>()?;
    this.sort_ascending()?;
    Ok(())
}

fn array_sort_desc(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_mut::<ScriptArray>()?;
    this.sort_descending()?;
    Ok(())
}

// ... additional helper function implementations
```

### Array Module Function Summary

| Category | Methods | Rust Source |
|----------|---------|-------------|
| **Size/Capacity** | length, isEmpty, resize, reserve, capacity, shrinkToFit, clear | `Vec::len`, `is_empty`, `resize`, `reserve`, `capacity`, `shrink_to_fit`, `clear` |
| **Access** | opIndex, first, last | `Vec::[]`, `first`, `last` |
| **Insertion** | insertAt, insertLast, insertFirst, extend | `Vec::insert`, `push`, `extend` |
| **Removal** | removeAt, removeLast, removeFirst, removeRange, popLast, popFirst, retain, dedup | `Vec::remove`, `pop`, `drain`, `retain`, `dedup` |
| **Search** | find, findByRef, rfind, contains, count | `iter().position()`, `contains`, `iter().filter().count()` |
| **Ordering** | reverse, sortAsc, sortDesc, isSorted, shuffle | `Vec::reverse`, `sort`, `sort_by`, `is_sorted` |
| **Transform** | fill, swap, rotate | `Vec::fill`, `swap`, `rotate_left/right` |
| **Slicing** | slice, sliceFrom, sliceTo, clone | `&[..]` slicing, `clone` |
| **Binary Search** | binarySearch | `Vec::binary_search` |

### dictionary.rs

```rust
// src/modules/dictionary.rs
//
// Dictionary template backed by Rust's HashMap<K, V>. Provides AngelScript
// standard library compatibility plus additional Rust HashMap methods.
//
// Note: AngelScript's standard dictionary uses string keys only. Our implementation
// extends this to support generic keys (dictionary<K, V>) where K is hashable.

use crate::ffi::{Module, FfiRegistrationError, TemplateValidation, CallContext, NativeError};
use crate::runtime::ScriptDict;

pub fn dictionary() -> Result<Module<'static>, FfiRegistrationError> {
    let mut module = Module::root();

    // =========================================================================
    // DICTIONARY<K, V> TYPE REGISTRATION
    // =========================================================================

    module.register_type::<ScriptDict>("dictionary<class K, class V>")
        .reference_type()
        .template_callback(|info| {
            // Keys must be hashable (primitives, string, handles)
            let key_type = &info.sub_types[0];
            if is_hashable(key_type) {
                TemplateValidation::valid()
            } else {
                TemplateValidation::invalid("Dictionary key must be hashable (primitive, string, or handle)")
            }
        })?

        // =====================================================================
        // FACTORIES
        // =====================================================================

        .factory("dictionary<K,V>@ f()", ScriptDict::new)?
        .factory("dictionary<K,V>@ f(uint capacity)", ScriptDict::with_capacity)?

        // Reference counting
        .addref(ScriptDict::add_ref)
        .release(ScriptDict::release)

        // =====================================================================
        // OPERATORS (AngelScript standard)
        // =====================================================================

        // Assignment
        .operator("dictionary<K,V>& opAssign(const dictionary<K,V> &in)", dict_assign)?

        // Index access (inserts default if not present)
        .operator("V& opIndex(const K &in)", dict_index)?
        .operator("const V& opIndex(const K &in) const", dict_index_const)?

        // =====================================================================
        // SIZE AND CAPACITY (AngelScript standard + Rust extras)
        // =====================================================================

        // AngelScript standard
        .method("uint getSize() const", ScriptDict::len)?                       // HashMap::len
        .method("bool isEmpty() const", ScriptDict::is_empty)?                  // HashMap::is_empty

        // Rust extras
        .method("uint capacity() const", ScriptDict::capacity)?                 // HashMap::capacity
        .method("void reserve(uint additional)", ScriptDict::reserve)?          // HashMap::reserve
        .method("void shrinkToFit()", ScriptDict::shrink_to_fit)?               // HashMap::shrink_to_fit

        // =====================================================================
        // INSERTION AND UPDATE (AngelScript standard + Rust extras)
        // =====================================================================

        // AngelScript standard
        .method("void set(const K &in key, const V &in value)", dict_set)?      // HashMap::insert

        // Rust extras
        .method("bool insert(const K &in key, const V &in value)", dict_insert)?  // Returns false if key existed
        .method("V getOrInsert(const K &in key, const V &in default)", dict_get_or_insert)?  // entry().or_insert()
        .method("bool tryInsert(const K &in key, const V &in value)", dict_try_insert)?  // Insert only if absent

        // =====================================================================
        // RETRIEVAL (AngelScript standard + Rust extras)
        // =====================================================================

        // AngelScript standard
        .method("bool get(const K &in key, V &out value) const", dict_get)?     // Returns true if found

        // Rust extras
        .method("V getOr(const K &in key, const V &in default) const", dict_get_or)?  // unwrap_or
        .method("bool tryGet(const K &in key, V &out value) const", dict_try_get)?    // Same as get, alias

        // =====================================================================
        // EXISTENCE AND DELETION (AngelScript standard + Rust extras)
        // =====================================================================

        // AngelScript standard
        .method("bool exists(const K &in key) const", dict_exists)?             // HashMap::contains_key
        .method("bool delete(const K &in key)", dict_delete)?                   // HashMap::remove (returns true if removed)
        .method("void deleteAll()", ScriptDict::clear)?                         // HashMap::clear (AngelScript name)
        .method("void clear()", ScriptDict::clear)?                             // Rust alias

        // Rust extras
        .method("V remove(const K &in key)", dict_remove)?                      // HashMap::remove with return value
        .method("bool removeIf(const K &in key, const V &in expected)", dict_remove_if)?  // Remove only if value matches

        // =====================================================================
        // KEY/VALUE ACCESS (AngelScript standard + Rust extras)
        // =====================================================================

        // AngelScript standard
        .method("array<K>@ getKeys() const", dict_get_keys)?                    // Collect keys into array

        // Rust extras
        .method("array<V>@ getValues() const", dict_get_values)?                // Collect values into array
        .method("array<K>@ keys() const", dict_get_keys)?                       // Alias
        .method("array<V>@ values() const", dict_get_values)?                   // Alias

        // =====================================================================
        // BULK OPERATIONS (Rust extras)
        // =====================================================================

        .method("void extend(const dictionary<K,V> &in other)", dict_extend)?   // Insert all from other
        .method("void retain(const array<K> &in keysToKeep)", dict_retain)?     // Keep only specified keys
        .method("dictionary<K,V>@ clone() const", dict_clone)?                  // Clone dictionary

        // =====================================================================
        // PREDICATES (Rust extras)
        // =====================================================================

        .method("bool containsValue(const V &in value) const", dict_contains_value)?  // Linear search for value
        .method("uint countValue(const V &in value) const", dict_count_value)?        // Count occurrences of value

        .build()?;

    Ok(module)
}

// =============================================================================
// VALIDATION
// =============================================================================

fn is_hashable(ty: &DataType) -> bool {
    // Primitives, strings, and handles are hashable
    // Value types with opEquals and opHash could also be supported
    ty.is_primitive() || ty.is_string() || ty.is_handle()
}

// =============================================================================
// HELPER FUNCTION IMPLEMENTATIONS
// =============================================================================

fn dict_assign(ctx: &mut CallContext) -> Result<(), NativeError> {
    let other = ctx.arg_dict(0)?;
    let this = ctx.this_mut::<ScriptDict>()?;
    this.clone_from(other);
    ctx.set_return_ref(this)?;
    Ok(())
}

fn dict_index(ctx: &mut CallContext) -> Result<(), NativeError> {
    let key = ctx.arg_any(0)?;
    let this = ctx.this_mut::<ScriptDict>()?;
    // Get or insert default
    let value = this.entry(key).or_insert_default()?;
    ctx.set_return_ref(value)?;
    Ok(())
}

fn dict_index_const(ctx: &mut CallContext) -> Result<(), NativeError> {
    let key = ctx.arg_any(0)?;
    let this = ctx.this_ref::<ScriptDict>()?;
    match this.get(&key) {
        Some(value) => ctx.set_return_ref(value)?,
        None => return Err(NativeError::KeyNotFound),
    }
    Ok(())
}

fn dict_set(ctx: &mut CallContext) -> Result<(), NativeError> {
    let key = ctx.arg_any(0)?;
    let value = ctx.arg_any(1)?;
    let this = ctx.this_mut::<ScriptDict>()?;
    this.insert(key, value);
    Ok(())
}

fn dict_get(ctx: &mut CallContext) -> Result<(), NativeError> {
    let key = ctx.arg_any(0)?;
    let out_value = ctx.arg_out_ref(1)?;
    let this = ctx.this_ref::<ScriptDict>()?;
    match this.get(&key) {
        Some(value) => {
            out_value.copy_from(value)?;
            ctx.set_return(true)?;
        }
        None => {
            ctx.set_return(false)?;
        }
    }
    Ok(())
}

fn dict_exists(ctx: &mut CallContext) -> Result<(), NativeError> {
    let key = ctx.arg_any(0)?;
    let this = ctx.this_ref::<ScriptDict>()?;
    ctx.set_return(this.contains_key(&key))?;
    Ok(())
}

fn dict_delete(ctx: &mut CallContext) -> Result<(), NativeError> {
    let key = ctx.arg_any(0)?;
    let this = ctx.this_mut::<ScriptDict>()?;
    let existed = this.remove(&key).is_some();
    ctx.set_return(existed)?;
    Ok(())
}

fn dict_get_keys(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_ref::<ScriptDict>()?;
    let keys = this.keys().collect::<ScriptArray>();
    ctx.set_return_handle(keys)?;
    Ok(())
}

fn dict_get_values(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_ref::<ScriptDict>()?;
    let values = this.values().collect::<ScriptArray>();
    ctx.set_return_handle(values)?;
    Ok(())
}

fn dict_get_or(ctx: &mut CallContext) -> Result<(), NativeError> {
    let key = ctx.arg_any(0)?;
    let default = ctx.arg_any(1)?;
    let this = ctx.this_ref::<ScriptDict>()?;
    let result = this.get(&key).cloned().unwrap_or(default);
    ctx.set_return_any(result)?;
    Ok(())
}

fn dict_clone(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_ref::<ScriptDict>()?;
    let cloned = this.clone();
    ctx.set_return_handle(cloned)?;
    Ok(())
}

// ... additional helper function implementations
```

### Dictionary Module Function Summary

| Category | Methods | Rust Source |
|----------|---------|-------------|
| **Size/Capacity** | getSize, isEmpty, capacity, reserve, shrinkToFit | `HashMap::len`, `is_empty`, `capacity`, `reserve`, `shrink_to_fit` |
| **Insertion** | set, insert, getOrInsert, tryInsert | `HashMap::insert`, `entry().or_insert()` |
| **Retrieval** | get, getOr, tryGet, opIndex | `HashMap::get`, `get().unwrap_or()` |
| **Existence** | exists | `HashMap::contains_key` |
| **Deletion** | delete, deleteAll, clear, remove, removeIf | `HashMap::remove`, `clear` |
| **Key/Value Access** | getKeys, getValues, keys, values | `keys()`, `values()` collected into arrays |
| **Bulk Operations** | extend, retain, clone | `HashMap::extend`, `retain`, `clone` |
| **Predicates** | containsValue, countValue | Linear search through values |

---

## List Construction Behaviors

To support initialization list syntax like `array<int> a = {1, 2, 3};`, we need list construction behaviors.

### Background

AngelScript supports initialization lists for constructing objects:

```angelscript
array<int> arr = {1, 2, 3, 4, 5};
dictionary@ d = {{"key1", 1}, {"key2", 2}};
MyValueType v = {10, "hello"};
```

Two behaviors handle this:
- `asBEHAVE_LIST_CONSTRUCT` - For value types (constructs in-place)
- `asBEHAVE_LIST_FACTORY` - For reference types (returns handle)

### ClassBuilder Methods

```rust
// For value types
module.register_type::<MyStruct>("MyStruct")
    .value_type(size, ValueTypeFlags::empty())
    .list_construct("void f(int &in list) {repeat int}", my_list_construct)?
    .build()?;

// For reference types (array)
module.register_type::<ScriptArray>("array<class T>")
    .reference_type()
    .list_factory("array<T>@ f(int &in list) {repeat T}", array_list_factory)?
    .build()?;

// For reference types (dictionary with key-value pairs)
module.register_type::<ScriptDict>("dictionary<class K, class V>")
    .reference_type()
    .list_factory("dictionary<K,V>@ f(int &in list) {repeat {K, V}}", dict_list_factory)?
    .build()?;
```

### List Pattern Syntax

The list pattern after the signature specifies the expected format:
- `{repeat T}` - Zero or more elements of type T
- `{int, string}` - Fixed sequence: int followed by string
- `{repeat {int, string}}` - Repeated pairs (for dictionary)

### Types

```rust
pub struct ListBehavior {
    pub native_fn: NativeFn,
    pub pattern: ListPattern,
}

pub enum ListPattern {
    Repeat(TypeExpr),           // {repeat T}
    Fixed(Vec<TypeExpr>),       // {int, string}
    RepeatFixed(Vec<TypeExpr>), // {repeat {int, string}}
}

// Extend Behaviors struct
pub struct Behaviors {
    pub factory: Option<NativeFn>,
    pub addref: Option<NativeFn>,
    pub release: Option<NativeFn>,
    pub construct: Option<NativeFn>,
    pub destruct: Option<NativeFn>,
    pub copy_construct: Option<NativeFn>,
    pub assign: Option<NativeFn>,
    pub list_construct: Option<ListBehavior>,   // NEW
    pub list_factory: Option<ListBehavior>,     // NEW
}
```

### ListBuffer Access

The native function receives a buffer containing the list data:

```rust
fn array_list_factory(ctx: &mut CallContext) -> Result<*mut ScriptArray, NativeError> {
    let list: &ListBuffer = ctx.arg(0)?;

    // Get element count
    let count = list.len();

    // Create array
    let mut arr = ScriptArray::new(ctx.type_id());

    // Read each element
    for i in 0..count {
        let elem = list.get(i)?;
        arr.push(elem);
    }

    Ok(Box::into_raw(Box::new(arr)))
}

fn dict_list_factory(ctx: &mut CallContext) -> Result<*mut ScriptDict, NativeError> {
    let list: &ListBuffer = ctx.arg(0)?;

    let mut dict = ScriptDict::new();

    // Each entry is a pair {K, V}
    for i in 0..list.len() {
        let pair = list.get_tuple(i)?;
        let key = pair.get(0)?;
        let value = pair.get(1)?;
        dict.insert(key, value);
    }

    Ok(Box::into_raw(Box::new(dict)))
}
```

### Updated Array Registration

```rust
module.register_type::<ScriptArray>("array<class T>")
    .reference_type()
    .template_callback(|_| TemplateValidation::valid())?

    // Factories
    .factory("array<T>@ f()", ScriptArray::new)?
    .factory("array<T>@ f(uint length)", ScriptArray::with_length)?
    .factory("array<T>@ f(uint length, const T &in value)", ScriptArray::filled)?

    // List factory for initialization lists: array<int> a = {1, 2, 3};
    .list_factory("array<T>@ f(int &in list) {repeat T}", array_list_factory)?

    // ... rest of registration
```

### Updated Dictionary Registration

```rust
module.register_type::<ScriptDict>("dictionary<class K, class V>")
    .reference_type()
    .template_callback(is_hashable)?

    // Factories
    .factory("dictionary<K,V>@ f()", ScriptDict::new)?
    .factory("dictionary<K,V>@ f(uint capacity)", ScriptDict::with_capacity)?

    // List factory for initialization lists: dictionary@ d = {{"a", 1}, {"b", 2}};
    .list_factory("dictionary<K,V>@ f(int &in list) {repeat {K, V}}", dict_list_factory)?

    // ... rest of registration
```

### Files to Modify

- `src/ast/decl_parser.rs` - Parse list pattern syntax `{...}`
- `src/ffi/types.rs` - Add `ListPattern`, `ListBehavior`, extend `Behaviors`
- `src/ffi/class_builder.rs` - Add `list_construct()`, `list_factory()` methods
- `src/ffi/native_fn.rs` - Add `ListBuffer` type
- `src/ffi/apply.rs` - Handle list behavior registration

### List Behavior Acceptance Criteria

- [ ] Parser handles `{repeat T}`, `{T, U}`, `{repeat {T, U}}` patterns
- [ ] `Behaviors` struct has `list_construct` and `list_factory` fields
- [ ] `ClassBuilder::list_construct()` registers for value types
- [ ] `ClassBuilder::list_factory()` registers for reference types
- [ ] `ListBuffer` provides typed access to initialization list data
- [ ] Array supports `array<int> a = {1, 2, 3}` syntax
- [ ] Dictionary supports `dictionary@ d = {{"k", v}}` syntax

---

### math.rs

```rust
// src/modules/math.rs
//
// Exposes Rust std math functions. All functions are direct wrappers around
// std methods - no custom implementations for numerical correctness.
// Additional functions (lerp, smoothstep, etc.) can be added via external crates.

use crate::ffi::{Module, FfiRegistrationError};

pub fn math() -> Result<Module<'static>, FfiRegistrationError> {
    let mut module = Module::new(&["math"]);

    // =========================================================================
    // CONSTANTS (std::f64::consts and std::f32::consts)
    // =========================================================================

    // Mathematical constants (f64)
    module.register_global_property("const double PI", std::f64::consts::PI)?;
    module.register_global_property("const double E", std::f64::consts::E)?;
    module.register_global_property("const double TAU", std::f64::consts::TAU)?;
    module.register_global_property("const double FRAC_PI_2", std::f64::consts::FRAC_PI_2)?;
    module.register_global_property("const double FRAC_PI_3", std::f64::consts::FRAC_PI_3)?;
    module.register_global_property("const double FRAC_PI_4", std::f64::consts::FRAC_PI_4)?;
    module.register_global_property("const double FRAC_PI_6", std::f64::consts::FRAC_PI_6)?;
    module.register_global_property("const double FRAC_PI_8", std::f64::consts::FRAC_PI_8)?;
    module.register_global_property("const double FRAC_1_PI", std::f64::consts::FRAC_1_PI)?;
    module.register_global_property("const double FRAC_2_PI", std::f64::consts::FRAC_2_PI)?;
    module.register_global_property("const double FRAC_2_SQRT_PI", std::f64::consts::FRAC_2_SQRT_PI)?;
    module.register_global_property("const double SQRT_2", std::f64::consts::SQRT_2)?;
    module.register_global_property("const double FRAC_1_SQRT_2", std::f64::consts::FRAC_1_SQRT_2)?;
    module.register_global_property("const double LN_2", std::f64::consts::LN_2)?;
    module.register_global_property("const double LN_10", std::f64::consts::LN_10)?;
    module.register_global_property("const double LOG2_E", std::f64::consts::LOG2_E)?;
    module.register_global_property("const double LOG2_10", std::f64::consts::LOG2_10)?;
    module.register_global_property("const double LOG10_E", std::f64::consts::LOG10_E)?;
    module.register_global_property("const double LOG10_2", std::f64::consts::LOG10_2)?;

    // Special f64 values
    module.register_global_property("const double INFINITY", f64::INFINITY)?;
    module.register_global_property("const double NEG_INFINITY", f64::NEG_INFINITY)?;
    module.register_global_property("const double NAN", f64::NAN)?;
    module.register_global_property("const double EPSILON", f64::EPSILON)?;
    module.register_global_property("const double DBL_MIN", f64::MIN)?;
    module.register_global_property("const double DBL_MAX", f64::MAX)?;
    module.register_global_property("const double DBL_MIN_POSITIVE", f64::MIN_POSITIVE)?;

    // Special f32 values
    module.register_global_property("const float FLT_INFINITY", f32::INFINITY)?;
    module.register_global_property("const float FLT_NEG_INFINITY", f32::NEG_INFINITY)?;
    module.register_global_property("const float FLT_NAN", f32::NAN)?;
    module.register_global_property("const float FLT_EPSILON", f32::EPSILON)?;
    module.register_global_property("const float FLT_MIN", f32::MIN)?;
    module.register_global_property("const float FLT_MAX", f32::MAX)?;
    module.register_global_property("const float FLT_MIN_POSITIVE", f32::MIN_POSITIVE)?;

    // =========================================================================
    // TRIGONOMETRIC (f64::sin, f64::cos, etc.)
    // =========================================================================

    // Basic trig (f64)
    module.register_fn("double sin(double x)", |x: f64| x.sin())?;
    module.register_fn("double cos(double x)", |x: f64| x.cos())?;
    module.register_fn("double tan(double x)", |x: f64| x.tan())?;

    // Basic trig (f32)
    module.register_fn("float sinf(float x)", |x: f32| x.sin())?;
    module.register_fn("float cosf(float x)", |x: f32| x.cos())?;
    module.register_fn("float tanf(float x)", |x: f32| x.tan())?;

    // Inverse trig (f64)
    module.register_fn("double asin(double x)", |x: f64| x.asin())?;
    module.register_fn("double acos(double x)", |x: f64| x.acos())?;
    module.register_fn("double atan(double x)", |x: f64| x.atan())?;
    module.register_fn("double atan2(double y, double x)", |y: f64, x: f64| y.atan2(x))?;

    // Inverse trig (f32)
    module.register_fn("float asinf(float x)", |x: f32| x.asin())?;
    module.register_fn("float acosf(float x)", |x: f32| x.acos())?;
    module.register_fn("float atanf(float x)", |x: f32| x.atan())?;
    module.register_fn("float atan2f(float y, float x)", |y: f32, x: f32| y.atan2(x))?;

    // sin_cos returns both sin and cos (f64)
    module.register_fn("void sin_cos(double x, double &out s, double &out c)",
        |x: f64, s: &mut f64, c: &mut f64| { let (sin, cos) = x.sin_cos(); *s = sin; *c = cos; })?;
    module.register_fn("void sin_cosf(float x, float &out s, float &out c)",
        |x: f32, s: &mut f32, c: &mut f32| { let (sin, cos) = x.sin_cos(); *s = sin; *c = cos; })?;

    // =========================================================================
    // HYPERBOLIC (f64::sinh, f64::cosh, etc.)
    // =========================================================================

    // Hyperbolic (f64)
    module.register_fn("double sinh(double x)", |x: f64| x.sinh())?;
    module.register_fn("double cosh(double x)", |x: f64| x.cosh())?;
    module.register_fn("double tanh(double x)", |x: f64| x.tanh())?;

    // Hyperbolic (f32)
    module.register_fn("float sinhf(float x)", |x: f32| x.sinh())?;
    module.register_fn("float coshf(float x)", |x: f32| x.cosh())?;
    module.register_fn("float tanhf(float x)", |x: f32| x.tanh())?;

    // Inverse hyperbolic (f64)
    module.register_fn("double asinh(double x)", |x: f64| x.asinh())?;
    module.register_fn("double acosh(double x)", |x: f64| x.acosh())?;
    module.register_fn("double atanh(double x)", |x: f64| x.atanh())?;

    // Inverse hyperbolic (f32)
    module.register_fn("float asinhf(float x)", |x: f32| x.asinh())?;
    module.register_fn("float acoshf(float x)", |x: f32| x.acosh())?;
    module.register_fn("float atanhf(float x)", |x: f32| x.atanh())?;

    // =========================================================================
    // EXPONENTIAL (f64::exp, f64::exp2, f64::exp_m1)
    // =========================================================================

    // Exponential (f64)
    module.register_fn("double exp(double x)", |x: f64| x.exp())?;
    module.register_fn("double exp2(double x)", |x: f64| x.exp2())?;
    module.register_fn("double exp_m1(double x)", |x: f64| x.exp_m1())?;

    // Exponential (f32)
    module.register_fn("float expf(float x)", |x: f32| x.exp())?;
    module.register_fn("float exp2f(float x)", |x: f32| x.exp2())?;
    module.register_fn("float exp_m1f(float x)", |x: f32| x.exp_m1())?;

    // =========================================================================
    // LOGARITHMIC (f64::ln, f64::log2, f64::log10, f64::log, f64::ln_1p)
    // =========================================================================

    // Logarithmic (f64)
    module.register_fn("double ln(double x)", |x: f64| x.ln())?;
    module.register_fn("double log(double x)", |x: f64| x.ln())?;  // Alias
    module.register_fn("double log2(double x)", |x: f64| x.log2())?;
    module.register_fn("double log10(double x)", |x: f64| x.log10())?;
    module.register_fn("double log_base(double x, double base)", |x: f64, base: f64| x.log(base))?;
    module.register_fn("double ln_1p(double x)", |x: f64| x.ln_1p())?;

    // Logarithmic (f32)
    module.register_fn("float lnf(float x)", |x: f32| x.ln())?;
    module.register_fn("float logf(float x)", |x: f32| x.ln())?;  // Alias
    module.register_fn("float log2f(float x)", |x: f32| x.log2())?;
    module.register_fn("float log10f(float x)", |x: f32| x.log10())?;
    module.register_fn("float log_basef(float x, float base)", |x: f32, base: f32| x.log(base))?;
    module.register_fn("float ln_1pf(float x)", |x: f32| x.ln_1p())?;

    // =========================================================================
    // POWER AND ROOTS (f64::powf, f64::powi, f64::sqrt, f64::cbrt, f64::hypot)
    // =========================================================================

    // Power (f64)
    module.register_fn("double pow(double base, double exp)", |base: f64, exp: f64| base.powf(exp))?;
    module.register_fn("double powi(double base, int exp)", |base: f64, exp: i32| base.powi(exp))?;

    // Power (f32)
    module.register_fn("float powf(float base, float exp)", |base: f32, exp: f32| base.powf(exp))?;
    module.register_fn("float powif(float base, int exp)", |base: f32, exp: i32| base.powi(exp))?;

    // Roots (f64)
    module.register_fn("double sqrt(double x)", |x: f64| x.sqrt())?;
    module.register_fn("double cbrt(double x)", |x: f64| x.cbrt())?;
    module.register_fn("double hypot(double x, double y)", |x: f64, y: f64| x.hypot(y))?;

    // Roots (f32)
    module.register_fn("float sqrtf(float x)", |x: f32| x.sqrt())?;
    module.register_fn("float cbrtf(float x)", |x: f32| x.cbrt())?;
    module.register_fn("float hypotf(float x, float y)", |x: f32, y: f32| x.hypot(y))?;

    // =========================================================================
    // ROUNDING (f64::floor, f64::ceil, f64::round, f64::trunc, f64::fract)
    // =========================================================================

    // Rounding (f64)
    module.register_fn("double floor(double x)", |x: f64| x.floor())?;
    module.register_fn("double ceil(double x)", |x: f64| x.ceil())?;
    module.register_fn("double round(double x)", |x: f64| x.round())?;
    module.register_fn("double trunc(double x)", |x: f64| x.trunc())?;
    module.register_fn("double fract(double x)", |x: f64| x.fract())?;

    // Rounding (f32)
    module.register_fn("float floorf(float x)", |x: f32| x.floor())?;
    module.register_fn("float ceilf(float x)", |x: f32| x.ceil())?;
    module.register_fn("float roundf(float x)", |x: f32| x.round())?;
    module.register_fn("float truncf(float x)", |x: f32| x.trunc())?;
    module.register_fn("float fractf(float x)", |x: f32| x.fract())?;

    // =========================================================================
    // ABSOLUTE VALUE AND SIGN (f64::abs, f64::signum, f64::copysign)
    // =========================================================================

    // Absolute value
    module.register_fn("double abs(double x)", |x: f64| x.abs())?;
    module.register_fn("float absf(float x)", |x: f32| x.abs())?;
    module.register_fn("int abs(int x)", |x: i32| x.abs())?;
    module.register_fn("int64 abs(int64 x)", |x: i64| x.abs())?;

    // Signum
    module.register_fn("double signum(double x)", |x: f64| x.signum())?;
    module.register_fn("float signumf(float x)", |x: f32| x.signum())?;
    module.register_fn("int signum(int x)", |x: i32| x.signum())?;
    module.register_fn("int64 signum(int64 x)", |x: i64| x.signum())?;

    // Copy sign
    module.register_fn("double copysign(double x, double y)", |x: f64, y: f64| x.copysign(y))?;
    module.register_fn("float copysignf(float x, float y)", |x: f32, y: f32| x.copysign(y))?;

    // =========================================================================
    // MIN/MAX/CLAMP (f64::min, f64::max, f64::clamp, etc.)
    // =========================================================================

    // Min
    module.register_fn("double min(double a, double b)", |a: f64, b: f64| a.min(b))?;
    module.register_fn("float minf(float a, float b)", |a: f32, b: f32| a.min(b))?;
    module.register_fn("int min(int a, int b)", |a: i32, b: i32| a.min(b))?;
    module.register_fn("int64 min(int64 a, int64 b)", |a: i64, b: i64| a.min(b))?;
    module.register_fn("uint min(uint a, uint b)", |a: u32, b: u32| a.min(b))?;
    module.register_fn("uint64 min(uint64 a, uint64 b)", |a: u64, b: u64| a.min(b))?;

    // Max
    module.register_fn("double max(double a, double b)", |a: f64, b: f64| a.max(b))?;
    module.register_fn("float maxf(float a, float b)", |a: f32, b: f32| a.max(b))?;
    module.register_fn("int max(int a, int b)", |a: i32, b: i32| a.max(b))?;
    module.register_fn("int64 max(int64 a, int64 b)", |a: i64, b: i64| a.max(b))?;
    module.register_fn("uint max(uint a, uint b)", |a: u32, b: u32| a.max(b))?;
    module.register_fn("uint64 max(uint64 a, uint64 b)", |a: u64, b: u64| a.max(b))?;

    // Clamp
    module.register_fn("double clamp(double x, double min, double max)",
        |x: f64, min: f64, max: f64| x.clamp(min, max))?;
    module.register_fn("float clampf(float x, float min, float max)",
        |x: f32, min: f32, max: f32| x.clamp(min, max))?;
    module.register_fn("int clamp(int x, int min, int max)",
        |x: i32, min: i32, max: i32| x.clamp(min, max))?;
    module.register_fn("int64 clamp(int64 x, int64 min, int64 max)",
        |x: i64, min: i64, max: i64| x.clamp(min, max))?;
    module.register_fn("uint clamp(uint x, uint min, uint max)",
        |x: u32, min: u32, max: u32| x.clamp(min, max))?;
    module.register_fn("uint64 clamp(uint64 x, uint64 min, uint64 max)",
        |x: u64, min: u64, max: u64| x.clamp(min, max))?;

    // =========================================================================
    // FLOATING POINT CLASSIFICATION (f64::is_nan, f64::is_finite, etc.)
    // =========================================================================

    // Classification (f64)
    module.register_fn("bool is_nan(double x)", |x: f64| x.is_nan())?;
    module.register_fn("bool is_infinite(double x)", |x: f64| x.is_infinite())?;
    module.register_fn("bool is_finite(double x)", |x: f64| x.is_finite())?;
    module.register_fn("bool is_normal(double x)", |x: f64| x.is_normal())?;
    module.register_fn("bool is_subnormal(double x)", |x: f64| x.is_subnormal())?;
    module.register_fn("bool is_sign_positive(double x)", |x: f64| x.is_sign_positive())?;
    module.register_fn("bool is_sign_negative(double x)", |x: f64| x.is_sign_negative())?;

    // Classification (f32)
    module.register_fn("bool is_nanf(float x)", |x: f32| x.is_nan())?;
    module.register_fn("bool is_infinitef(float x)", |x: f32| x.is_infinite())?;
    module.register_fn("bool is_finitef(float x)", |x: f32| x.is_finite())?;
    module.register_fn("bool is_normalf(float x)", |x: f32| x.is_normal())?;
    module.register_fn("bool is_subnormalf(float x)", |x: f32| x.is_subnormal())?;
    module.register_fn("bool is_sign_positivef(float x)", |x: f32| x.is_sign_positive())?;
    module.register_fn("bool is_sign_negativef(float x)", |x: f32| x.is_sign_negative())?;

    // =========================================================================
    // FUSED MULTIPLY-ADD (f64::mul_add)
    // =========================================================================

    module.register_fn("double mul_add(double x, double a, double b)",
        |x: f64, a: f64, b: f64| x.mul_add(a, b))?;
    module.register_fn("float mul_addf(float x, float a, float b)",
        |x: f32, a: f32, b: f32| x.mul_add(a, b))?;

    // =========================================================================
    // EUCLIDEAN DIVISION AND REMAINDER (f64::div_euclid, f64::rem_euclid)
    // =========================================================================

    // Euclidean division (f64)
    module.register_fn("double div_euclid(double x, double y)", |x: f64, y: f64| x.div_euclid(y))?;
    module.register_fn("double rem_euclid(double x, double y)", |x: f64, y: f64| x.rem_euclid(y))?;

    // Euclidean division (f32)
    module.register_fn("float div_euclidf(float x, float y)", |x: f32, y: f32| x.div_euclid(y))?;
    module.register_fn("float rem_euclidf(float x, float y)", |x: f32, y: f32| x.rem_euclid(y))?;

    // Euclidean division (integers)
    module.register_fn("int div_euclid(int x, int y)", |x: i32, y: i32| x.div_euclid(y))?;
    module.register_fn("int rem_euclid(int x, int y)", |x: i32, y: i32| x.rem_euclid(y))?;
    module.register_fn("int64 div_euclid(int64 x, int64 y)", |x: i64, y: i64| x.div_euclid(y))?;
    module.register_fn("int64 rem_euclid(int64 x, int64 y)", |x: i64, y: i64| x.rem_euclid(y))?;

    // =========================================================================
    // ANGLE CONVERSION (f64::to_radians, f64::to_degrees)
    // =========================================================================

    module.register_fn("double to_radians(double degrees)", |d: f64| d.to_radians())?;
    module.register_fn("double to_degrees(double radians)", |r: f64| r.to_degrees())?;
    module.register_fn("float to_radiansf(float degrees)", |d: f32| d.to_radians())?;
    module.register_fn("float to_degreesf(float radians)", |r: f32| r.to_degrees())?;

    // =========================================================================
    // BIT CONVERSION (f64::to_bits, f64::from_bits)
    // =========================================================================

    module.register_fn("uint64 to_bits(double x)", |x: f64| x.to_bits())?;
    module.register_fn("double from_bits(uint64 bits)", f64::from_bits)?;
    module.register_fn("uint to_bitsf(float x)", |x: f32| x.to_bits())?;
    module.register_fn("float from_bitsf(uint bits)", f32::from_bits)?;

    // =========================================================================
    // MIDPOINT (f64::midpoint - available in Rust 1.85+)
    // =========================================================================
    // Note: midpoint was stabilized in Rust 1.85. Uncomment when available:
    // module.register_fn("double midpoint(double a, double b)", |a: f64, b: f64| f64::midpoint(a, b))?;
    // module.register_fn("float midpointf(float a, float b)", |a: f32, b: f32| f32::midpoint(a, b))?;

    Ok(module)
}
```

### Math Module Function Summary

All functions are direct wrappers around Rust std methods.

| Category | Functions | Rust Source |
|----------|-----------|-------------|
| **Constants (f64)** | PI, E, TAU, FRAC_PI_2/3/4/6/8, FRAC_1_PI, FRAC_2_PI, FRAC_2_SQRT_PI, SQRT_2, FRAC_1_SQRT_2, LN_2, LN_10, LOG2_E, LOG2_10, LOG10_E, LOG10_2 | `std::f64::consts` |
| **Special Values (f64)** | INFINITY, NEG_INFINITY, NAN, EPSILON, DBL_MIN, DBL_MAX, DBL_MIN_POSITIVE | `f64::*` |
| **Special Values (f32)** | FLT_INFINITY, FLT_NEG_INFINITY, FLT_NAN, FLT_EPSILON, FLT_MIN, FLT_MAX, FLT_MIN_POSITIVE | `f32::*` |
| **Trigonometric** | sin, cos, tan, asin, acos, atan, atan2, sin_cos (+ f32 variants) | `f64::sin`, etc. |
| **Hyperbolic** | sinh, cosh, tanh, asinh, acosh, atanh (+ f32 variants) | `f64::sinh`, etc. |
| **Exponential** | exp, exp2, exp_m1 (+ f32 variants) | `f64::exp`, etc. |
| **Logarithmic** | ln, log, log2, log10, log_base, ln_1p (+ f32 variants) | `f64::ln`, etc. |
| **Power/Roots** | pow, powi, sqrt, cbrt, hypot (+ f32 variants) | `f64::powf`, etc. |
| **Rounding** | floor, ceil, round, trunc, fract (+ f32 variants) | `f64::floor`, etc. |
| **Absolute/Sign** | abs, signum, copysign (f64/f32/i32/i64) | `f64::abs`, etc. |
| **Min/Max/Clamp** | min, max, clamp (f64/f32/i32/i64/u32/u64) | `f64::min`, etc. |
| **Classification** | is_nan, is_infinite, is_finite, is_normal, is_subnormal, is_sign_positive, is_sign_negative (+ f32 variants) | `f64::is_nan`, etc. |
| **FMA** | mul_add (f64/f32) | `f64::mul_add` |
| **Euclidean** | div_euclid, rem_euclid (f64/f32/i32/i64) | `f64::div_euclid`, etc. |
| **Angle Conversion** | to_radians, to_degrees (+ f32 variants) | `f64::to_radians`, etc. |
| **Bit Conversion** | to_bits, from_bits (f64/f32) | `f64::to_bits`, etc. |

### Future Extensions

Additional math functions can be added via external crates for numerical correctness:
- **Interpolation**: lerp, smoothstep, inverse_lerp (consider `num-traits` or `glam`)
- **Random**: random number generation (consider `rand` crate)
- **Complex numbers**: (consider `num-complex` crate)
- **Special functions**: gamma, erf, bessel (consider `statrs` or `special` crates)

## Registry Cleanup

Remove from `src/semantic/types/registry.rs`:
- `register_builtin_string()` (~400 lines)
- `register_builtin_template()` for array/dictionary
- All hardcoded method/operator registration

## Usage

```rust
use angelscript::modules;

// Install default modules (all built-ins)
let mut ctx = Context::new();  // Automatically installs default_modules()

// Or install selectively
let mut ctx = Context::new_raw();  // No built-ins
ctx.install(modules::string()?)?;
ctx.install(modules::array()?)?;
ctx.install(modules::math()?)?;  // math::sin(), math::cos(), etc.
```

## Acceptance Criteria

- [ ] All built-in types work through FFI registration
- [ ] Declaration strings parse correctly for all methods
- [ ] Existing tests pass with new implementation
- [ ] Registry.rs reduced by ~800+ lines
- [ ] Context::new() installs all default modules
- [ ] Context::new_raw() creates context without built-ins
- [ ] Individual modules can be installed selectively
- [ ] math namespace works correctly (math::sin, math::PI)
