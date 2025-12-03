# Task 08h: Array Module

**Status:** Not Started
**Parent:** Task 08 - Built-in Modules
**Depends On:** 08b (ScriptArray), 08d (modules), 08g (list behaviors)

---

## Objective

Register the `array<T>` template type via FFI using the ClassBuilder API with all methods and list factory support.

## Files to Create/Modify

- `src/modules/array.rs` - Array template registration (new)
- Update `src/modules/mod.rs` - Add export

## Implementation

### src/modules/array.rs

```rust
//! Array template registration.
//!
//! Registers the built-in `array<T>` template type with all methods and operators.

use crate::ffi::{Module, FfiRegistrationError, TemplateValidation, CallContext, NativeError};
use crate::runtime::ScriptArray;

/// Creates the array module with the array<T> template.
pub fn array_module<'app>() -> Result<Module<'app>, FfiRegistrationError> {
    let mut module = Module::root();

    // =========================================================================
    // ARRAY<T> TEMPLATE REGISTRATION
    // =========================================================================

    module.register_type::<ScriptArray>("array<class T>")
        .reference_type()

        // Template validation - arrays can contain any type
        .template_callback(|_info| TemplateValidation::valid())?

        // =====================================================================
        // FACTORIES
        // =====================================================================

        .factory("array<T>@ f()", array_new)?
        .factory("array<T>@ f(uint length)", array_with_length)?
        .factory("array<T>@ f(uint length, const T &in value)", array_filled)?

        // List factory for initialization lists: array<int> a = {1, 2, 3}
        .list_factory("array<T>@ f(int &in list) {repeat T}", array_list_factory)?

        // Reference counting
        .addref(ScriptArray::add_ref)
        .release(ScriptArray::release)

        // =====================================================================
        // OPERATORS
        // =====================================================================

        .operator("array<T>& opAssign(const array<T> &in)", array_assign)?
        .operator("bool opEquals(const array<T> &in) const", array_equals)?
        .operator("T& opIndex(uint)", array_index)?
        .operator("const T& opIndex(uint) const", array_index_const)?

        // =====================================================================
        // SIZE AND CAPACITY
        // =====================================================================

        .method("uint length() const", array_length)?
        .method("bool isEmpty() const", array_is_empty)?
        .method("uint capacity() const", array_capacity)?
        .method("void resize(uint length)", array_resize)?
        .method("void reserve(uint length)", array_reserve)?
        .method("void shrinkToFit()", array_shrink_to_fit)?
        .method("void clear()", array_clear)?

        // =====================================================================
        // ELEMENT ACCESS
        // =====================================================================

        .method("T& first()", array_first)?
        .method("const T& first() const", array_first_const)?
        .method("T& last()", array_last)?
        .method("const T& last() const", array_last_const)?

        // =====================================================================
        // INSERTION
        // =====================================================================

        .method("void insertAt(uint index, const T &in value)", array_insert_at)?
        .method("void insertAt(uint index, const array<T> &in arr)", array_insert_array_at)?
        .method("void insertLast(const T &in value)", array_insert_last)?
        .method("void insertFirst(const T &in value)", array_insert_first)?
        .method("void extend(const array<T> &in arr)", array_extend)?

        // =====================================================================
        // REMOVAL
        // =====================================================================

        .method("void removeAt(uint index)", array_remove_at)?
        .method("void removeLast()", array_remove_last)?
        .method("void removeFirst()", array_remove_first)?
        .method("void removeRange(uint start, uint count)", array_remove_range)?
        .method("T popLast()", array_pop_last)?
        .method("T popFirst()", array_pop_first)?
        .method("void dedup()", array_dedup)?

        // =====================================================================
        // SEARCH
        // =====================================================================

        .method("int find(const T &in value) const", array_find)?
        .method("int find(uint startAt, const T &in value) const", array_find_from)?
        .method("int findByRef(const T &in value) const", array_find_by_ref)?
        .method("int findByRef(uint startAt, const T &in value) const", array_find_by_ref_from)?
        .method("int rfind(const T &in value) const", array_rfind)?
        .method("bool contains(const T &in value) const", array_contains)?
        .method("uint count(const T &in value) const", array_count)?

        // =====================================================================
        // ORDERING
        // =====================================================================

        .method("void reverse()", array_reverse)?
        .method("void sortAsc()", array_sort_asc)?
        .method("void sortAsc(uint startAt, uint count)", array_sort_asc_range)?
        .method("void sortDesc()", array_sort_desc)?
        .method("void sortDesc(uint startAt, uint count)", array_sort_desc_range)?
        .method("bool isSorted() const", array_is_sorted)?
        .method("bool isSortedDesc() const", array_is_sorted_desc)?

        // =====================================================================
        // TRANSFORMATION
        // =====================================================================

        .method("void fill(const T &in value)", array_fill)?
        .method("void swap(uint i, uint j)", array_swap)?
        .method("void rotate(int amount)", array_rotate)?

        // =====================================================================
        // SLICING
        // =====================================================================

        .method("array<T>@ slice(uint start, uint end) const", array_slice)?
        .method("array<T>@ sliceFrom(uint start) const", array_slice_from)?
        .method("array<T>@ sliceTo(uint end) const", array_slice_to)?
        .method("array<T>@ clone() const", array_clone)?

        // =====================================================================
        // BINARY SEARCH
        // =====================================================================

        .method("int binarySearch(const T &in value) const", array_binary_search)?

        .build()?;

    Ok(module)
}

// =============================================================================
// FACTORY IMPLEMENTATIONS
// =============================================================================

fn array_new(ctx: &mut CallContext) -> Result<(), NativeError> {
    let element_type = ctx.template_type(0)?;
    let arr = ScriptArray::new(element_type);
    ctx.set_return_handle(arr)?;
    Ok(())
}

fn array_with_length(ctx: &mut CallContext) -> Result<(), NativeError> {
    let length: u32 = ctx.arg(0)?;
    let element_type = ctx.template_type(0)?;
    let arr = ScriptArray::with_length(element_type, length as usize);
    ctx.set_return_handle(arr)?;
    Ok(())
}

fn array_filled(ctx: &mut CallContext) -> Result<(), NativeError> {
    let length: u32 = ctx.arg(0)?;
    let value = ctx.arg_any(1)?;
    let element_type = ctx.template_type(0)?;
    let arr = ScriptArray::filled(element_type, length as usize, value);
    ctx.set_return_handle(arr)?;
    Ok(())
}

fn array_list_factory(ctx: &mut CallContext) -> Result<(), NativeError> {
    let list = ctx.arg_list(0)?;
    let element_type = ctx.template_type(0)?;

    let mut arr = ScriptArray::new(element_type);
    for slot in list.iter() {
        arr.push(slot.clone());
    }

    ctx.set_return_handle(arr)?;
    Ok(())
}

// =============================================================================
// OPERATOR IMPLEMENTATIONS
// =============================================================================

fn array_assign(ctx: &mut CallContext) -> Result<(), NativeError> {
    let other = ctx.arg_handle::<ScriptArray>(0)?;
    let this = ctx.this_mut::<ScriptArray>()?;
    this.clone_from(&other);
    ctx.set_return_ref(this)?;
    Ok(())
}

fn array_equals(ctx: &mut CallContext) -> Result<(), NativeError> {
    let other = ctx.arg_handle::<ScriptArray>(0)?;
    let this = ctx.this_ref::<ScriptArray>()?;
    ctx.set_return(this.eq(&other))?;
    Ok(())
}

fn array_index(ctx: &mut CallContext) -> Result<(), NativeError> {
    let index: u32 = ctx.arg(0)?;
    let this = ctx.this_mut::<ScriptArray>()?;
    let elem = this.get_mut(index as usize)
        .ok_or(NativeError::IndexOutOfBounds)?;
    ctx.set_return_ref(elem)?;
    Ok(())
}

fn array_index_const(ctx: &mut CallContext) -> Result<(), NativeError> {
    let index: u32 = ctx.arg(0)?;
    let this = ctx.this_ref::<ScriptArray>()?;
    let elem = this.get(index as usize)
        .ok_or(NativeError::IndexOutOfBounds)?;
    ctx.set_return_ref(elem)?;
    Ok(())
}

// =============================================================================
// SIZE AND CAPACITY IMPLEMENTATIONS
// =============================================================================

fn array_length(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_ref::<ScriptArray>()?;
    ctx.set_return(this.len())?;
    Ok(())
}

fn array_is_empty(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_ref::<ScriptArray>()?;
    ctx.set_return(this.is_empty())?;
    Ok(())
}

fn array_capacity(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_ref::<ScriptArray>()?;
    ctx.set_return(this.capacity())?;
    Ok(())
}

fn array_resize(ctx: &mut CallContext) -> Result<(), NativeError> {
    let length: u32 = ctx.arg(0)?;
    let this = ctx.this_mut::<ScriptArray>()?;
    this.resize_with_default(length as usize);
    Ok(())
}

fn array_reserve(ctx: &mut CallContext) -> Result<(), NativeError> {
    let length: u32 = ctx.arg(0)?;
    let this = ctx.this_mut::<ScriptArray>()?;
    this.reserve(length as usize);
    Ok(())
}

fn array_shrink_to_fit(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_mut::<ScriptArray>()?;
    this.shrink_to_fit();
    Ok(())
}

fn array_clear(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_mut::<ScriptArray>()?;
    this.clear();
    Ok(())
}

// =============================================================================
// ELEMENT ACCESS IMPLEMENTATIONS
// =============================================================================

fn array_first(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_mut::<ScriptArray>()?;
    let elem = this.first_mut().ok_or(NativeError::IndexOutOfBounds)?;
    ctx.set_return_ref(elem)?;
    Ok(())
}

fn array_first_const(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_ref::<ScriptArray>()?;
    let elem = this.first().ok_or(NativeError::IndexOutOfBounds)?;
    ctx.set_return_ref(elem)?;
    Ok(())
}

fn array_last(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_mut::<ScriptArray>()?;
    let elem = this.last_mut().ok_or(NativeError::IndexOutOfBounds)?;
    ctx.set_return_ref(elem)?;
    Ok(())
}

fn array_last_const(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_ref::<ScriptArray>()?;
    let elem = this.last().ok_or(NativeError::IndexOutOfBounds)?;
    ctx.set_return_ref(elem)?;
    Ok(())
}

// =============================================================================
// INSERTION IMPLEMENTATIONS
// =============================================================================

fn array_insert_at(ctx: &mut CallContext) -> Result<(), NativeError> {
    let index: u32 = ctx.arg(0)?;
    let value = ctx.arg_any(1)?;
    let this = ctx.this_mut::<ScriptArray>()?;
    this.insert(index as usize, value);
    Ok(())
}

fn array_insert_array_at(ctx: &mut CallContext) -> Result<(), NativeError> {
    let index: u32 = ctx.arg(0)?;
    let other = ctx.arg_handle::<ScriptArray>(1)?;
    let this = ctx.this_mut::<ScriptArray>()?;
    this.insert_array(index as usize, &other);
    Ok(())
}

fn array_insert_last(ctx: &mut CallContext) -> Result<(), NativeError> {
    let value = ctx.arg_any(0)?;
    let this = ctx.this_mut::<ScriptArray>()?;
    this.push(value);
    Ok(())
}

fn array_insert_first(ctx: &mut CallContext) -> Result<(), NativeError> {
    let value = ctx.arg_any(0)?;
    let this = ctx.this_mut::<ScriptArray>()?;
    this.insert(0, value);
    Ok(())
}

fn array_extend(ctx: &mut CallContext) -> Result<(), NativeError> {
    let other = ctx.arg_handle::<ScriptArray>(0)?;
    let this = ctx.this_mut::<ScriptArray>()?;
    this.extend(&other);
    Ok(())
}

// =============================================================================
// REMOVAL IMPLEMENTATIONS
// =============================================================================

fn array_remove_at(ctx: &mut CallContext) -> Result<(), NativeError> {
    let index: u32 = ctx.arg(0)?;
    let this = ctx.this_mut::<ScriptArray>()?;
    this.remove_at(index as usize);
    Ok(())
}

fn array_remove_last(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_mut::<ScriptArray>()?;
    this.pop();
    Ok(())
}

fn array_remove_first(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_mut::<ScriptArray>()?;
    this.remove_at(0);
    Ok(())
}

fn array_remove_range(ctx: &mut CallContext) -> Result<(), NativeError> {
    let start: u32 = ctx.arg(0)?;
    let count: u32 = ctx.arg(1)?;
    let this = ctx.this_mut::<ScriptArray>()?;
    this.remove_range(start as usize, count as usize);
    Ok(())
}

fn array_pop_last(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_mut::<ScriptArray>()?;
    let value = this.pop().ok_or(NativeError::IndexOutOfBounds)?;
    ctx.set_return_any(value)?;
    Ok(())
}

fn array_pop_first(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_mut::<ScriptArray>()?;
    let value = this.remove_at(0).ok_or(NativeError::IndexOutOfBounds)?;
    ctx.set_return_any(value)?;
    Ok(())
}

fn array_dedup(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_mut::<ScriptArray>()?;
    this.dedup();
    Ok(())
}

// =============================================================================
// SEARCH IMPLEMENTATIONS
// =============================================================================

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

fn array_find_by_ref(ctx: &mut CallContext) -> Result<(), NativeError> {
    // findByRef compares by reference/handle identity, not value
    let value = ctx.arg_any(0)?;
    let this = ctx.this_ref::<ScriptArray>()?;
    let result = this.find_by_ref(&value).map(|i| i as i32).unwrap_or(-1);
    ctx.set_return(result)?;
    Ok(())
}

fn array_find_by_ref_from(ctx: &mut CallContext) -> Result<(), NativeError> {
    let start: u32 = ctx.arg(0)?;
    let value = ctx.arg_any(1)?;
    let this = ctx.this_ref::<ScriptArray>()?;
    let result = this.find_by_ref_from(start as usize, &value).map(|i| i as i32).unwrap_or(-1);
    ctx.set_return(result)?;
    Ok(())
}

fn array_rfind(ctx: &mut CallContext) -> Result<(), NativeError> {
    let value = ctx.arg_any(0)?;
    let this = ctx.this_ref::<ScriptArray>()?;
    let result = this.rfind(&value).map(|i| i as i32).unwrap_or(-1);
    ctx.set_return(result)?;
    Ok(())
}

fn array_contains(ctx: &mut CallContext) -> Result<(), NativeError> {
    let value = ctx.arg_any(0)?;
    let this = ctx.this_ref::<ScriptArray>()?;
    ctx.set_return(this.contains(&value))?;
    Ok(())
}

fn array_count(ctx: &mut CallContext) -> Result<(), NativeError> {
    let value = ctx.arg_any(0)?;
    let this = ctx.this_ref::<ScriptArray>()?;
    ctx.set_return(this.count(&value))?;
    Ok(())
}

// =============================================================================
// ORDERING IMPLEMENTATIONS
// =============================================================================

fn array_reverse(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_mut::<ScriptArray>()?;
    this.reverse();
    Ok(())
}

fn array_sort_asc(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_mut::<ScriptArray>()?;
    this.sort_ascending();
    Ok(())
}

fn array_sort_asc_range(ctx: &mut CallContext) -> Result<(), NativeError> {
    let start: u32 = ctx.arg(0)?;
    let count: u32 = ctx.arg(1)?;
    let this = ctx.this_mut::<ScriptArray>()?;
    this.sort_ascending_range(start as usize, count as usize);
    Ok(())
}

fn array_sort_desc(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_mut::<ScriptArray>()?;
    this.sort_descending();
    Ok(())
}

fn array_sort_desc_range(ctx: &mut CallContext) -> Result<(), NativeError> {
    let start: u32 = ctx.arg(0)?;
    let count: u32 = ctx.arg(1)?;
    let this = ctx.this_mut::<ScriptArray>()?;
    this.sort_descending_range(start as usize, count as usize);
    Ok(())
}

fn array_is_sorted(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_ref::<ScriptArray>()?;
    ctx.set_return(this.is_sorted())?;
    Ok(())
}

fn array_is_sorted_desc(ctx: &mut CallContext) -> Result<(), NativeError> {
    let this = ctx.this_ref::<ScriptArray>()?;
    ctx.set_return(this.is_sorted_desc())?;
    Ok(())
}

// ... Additional implementations for remaining methods
```

## Method Summary

| Category | Methods |
|----------|---------|
| **Factories** | f(), f(uint), f(uint, T&in), list factory |
| **Lifecycle** | addref, release |
| **Operators** | opAssign, opEquals, opIndex (const + mutable) |
| **Size** | length, isEmpty, capacity, resize, reserve, shrinkToFit, clear |
| **Access** | first, last (const + mutable) |
| **Insert** | insertAt, insertLast, insertFirst, extend |
| **Remove** | removeAt, removeLast, removeFirst, removeRange, popLast, popFirst, dedup |
| **Search** | find, findByRef, rfind, contains, count |
| **Order** | reverse, sortAsc, sortDesc, isSorted |
| **Transform** | fill, swap, rotate |
| **Slice** | slice, sliceFrom, sliceTo, clone |
| **Binary** | binarySearch |

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_array_module_builds() {
        let module = array_module().expect("array module should build");
        // Should have array template registered
        assert!(module.types().iter().any(|t| t.name() == "array"));
    }
}
```

## Acceptance Criteria

- [ ] `src/modules/array.rs` created
- [ ] Array template registered as reference type
- [ ] All factories registered including list factory
- [ ] AddRef/Release behaviors registered
- [ ] All operators registered (4)
- [ ] All methods registered (~35)
- [ ] Template callback accepts any T
- [ ] Unit tests pass
- [ ] `cargo build --lib` succeeds
