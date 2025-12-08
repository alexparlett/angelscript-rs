//! ScriptArray - Type-erased, reference-counted array for AngelScript.
//!
//! This is a REFERENCE type - passed by handle with manual reference counting.
//! Elements are stored type-erased as raw bytes.

use std::fmt;
use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};

use angelscript_core::TypeHash;
use angelscript_macros::Any;
use angelscript_registry::Module;

/// Type-erased array for AngelScript `array<T>` template.
///
/// This is a REFERENCE type with manual reference counting.
/// Elements are stored as raw bytes for type-erased storage.
#[derive(Any)]
#[angelscript(name = "array", reference, template = "<T>")]
pub struct ScriptArray {
    /// Type-erased element storage (raw bytes)
    elements: Vec<u8>,
    /// Size of each element in bytes
    element_size: usize,
    /// Number of elements
    len: u32,
    /// Element type for runtime checking
    element_type_id: TypeHash,
    /// Reference count (starts at 1)
    ref_count: AtomicU32,
}

impl ScriptArray {
    // =========================================================================
    // CONSTRUCTORS
    // =========================================================================

    /// Create an empty array for given element type and size.
    pub fn new(element_type_id: TypeHash, element_size: usize) -> Self {
        Self {
            elements: Vec::new(),
            element_size,
            len: 0,
            element_type_id,
            ref_count: AtomicU32::new(1),
        }
    }

    /// Create array with initial capacity.
    pub fn with_capacity(element_type_id: TypeHash, element_size: usize, capacity: usize) -> Self {
        Self {
            elements: Vec::with_capacity(capacity * element_size),
            element_size,
            len: 0,
            element_type_id,
            ref_count: AtomicU32::new(1),
        }
    }

    /// Get the element type ID.
    #[inline]
    pub fn element_type_id(&self) -> TypeHash {
        self.element_type_id
    }

    /// Get the element size in bytes.
    #[inline]
    pub fn element_size(&self) -> usize {
        self.element_size
    }

    // =========================================================================
    // REFERENCE COUNTING
    // =========================================================================

    /// Increment reference count.
    #[angelscript_macros::function(addref)]
    pub fn add_ref(&self) {
        self.ref_count.fetch_add(1, AtomicOrdering::Relaxed);
    }

    /// Decrement reference count. Returns true if count reached zero.
    #[angelscript_macros::function(release)]
    pub fn release(&self) -> bool {
        self.ref_count.fetch_sub(1, AtomicOrdering::Release) == 1
    }

    /// Get current reference count.
    #[inline]
    pub fn ref_count(&self) -> u32 {
        self.ref_count.load(AtomicOrdering::Relaxed)
    }

    // =========================================================================
    // SIZE AND CAPACITY
    // =========================================================================

    /// Returns the number of elements.
    #[angelscript_macros::function(instance, const, name = "length")]
    pub fn len(&self) -> u32 {
        self.len
    }

    /// Returns true if the array is empty.
    #[angelscript_macros::function(instance, const, name = "isEmpty")]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the allocated capacity.
    #[angelscript_macros::function(instance, const)]
    pub fn capacity(&self) -> u32 {
        if self.element_size == 0 {
            u32::MAX
        } else {
            (self.elements.capacity() / self.element_size) as u32
        }
    }

    /// Reserve capacity for at least `additional` more elements.
    #[angelscript_macros::function(instance)]
    pub fn reserve(&mut self, additional: u32) {
        self.elements.reserve(additional as usize * self.element_size);
    }

    /// Shrink capacity to fit current length.
    #[angelscript_macros::function(instance, name = "shrinkToFit")]
    pub fn shrink_to_fit(&mut self) {
        self.elements.shrink_to_fit();
    }

    /// Remove all elements.
    #[angelscript_macros::function(instance)]
    pub fn clear(&mut self) {
        self.elements.clear();
        self.len = 0;
    }

    /// Resize array to `new_len` elements.
    /// New elements are zero-initialized.
    #[angelscript_macros::function(instance)]
    pub fn resize(&mut self, new_len: u32) {
        let new_byte_len = new_len as usize * self.element_size;
        self.elements.resize(new_byte_len, 0);
        self.len = new_len;
    }

    // =========================================================================
    // RAW ELEMENT ACCESS (for VM use)
    // =========================================================================

    /// Get raw pointer to element at index.
    ///
    /// # Safety
    /// Index must be within bounds.
    #[inline]
    pub unsafe fn get_raw(&self, index: u32) -> *const u8 {
        unsafe {
            self.elements
                .as_ptr()
                .add(index as usize * self.element_size)
        }
    }

    /// Get mutable raw pointer to element at index.
    ///
    /// # Safety
    /// Index must be within bounds.
    #[inline]
    pub unsafe fn get_raw_mut(&mut self, index: u32) -> *mut u8 {
        unsafe {
            self.elements
                .as_mut_ptr()
                .add(index as usize * self.element_size)
        }
    }

    /// Push raw bytes as a new element.
    ///
    /// # Safety
    /// The bytes must represent a valid value of the element type.
    pub unsafe fn push_raw(&mut self, bytes: &[u8]) {
        debug_assert_eq!(bytes.len(), self.element_size);
        self.elements.extend_from_slice(bytes);
        self.len += 1;
    }

    /// Pop the last element, returning its raw bytes.
    pub fn pop_raw(&mut self) -> Option<Vec<u8>> {
        if self.len == 0 {
            return None;
        }
        let start = (self.len as usize - 1) * self.element_size;
        let bytes = self.elements[start..].to_vec();
        self.elements.truncate(start);
        self.len -= 1;
        Some(bytes)
    }

    // =========================================================================
    // ORDERING
    // =========================================================================

    /// Reverse elements in place.
    #[angelscript_macros::function(instance)]
    pub fn reverse(&mut self) {
        if self.element_size == 0 || self.len <= 1 {
            return;
        }
        // Reverse by swapping elements
        let mut i = 0;
        let mut j = self.len - 1;
        while i < j {
            self.swap_raw(i, j);
            i += 1;
            j -= 1;
        }
    }

    /// Swap two elements by index.
    fn swap_raw(&mut self, i: u32, j: u32) {
        if i == j || i >= self.len || j >= self.len {
            return;
        }
        let i_start = i as usize * self.element_size;
        let j_start = j as usize * self.element_size;
        for k in 0..self.element_size {
            self.elements.swap(i_start + k, j_start + k);
        }
    }

    // =========================================================================
    // REMOVAL
    // =========================================================================

    /// Remove element at position.
    #[angelscript_macros::function(instance, name = "removeAt")]
    pub fn remove_at(&mut self, index: u32) {
        if index >= self.len {
            return;
        }
        let start = index as usize * self.element_size;
        let end = start + self.element_size;
        self.elements.drain(start..end);
        self.len -= 1;
    }

    /// Remove the last element.
    #[angelscript_macros::function(instance, name = "removeLast")]
    pub fn remove_last(&mut self) {
        if self.len > 0 {
            let new_byte_len = (self.len as usize - 1) * self.element_size;
            self.elements.truncate(new_byte_len);
            self.len -= 1;
        }
    }

    /// Remove range of elements [start..start+count].
    #[angelscript_macros::function(instance, name = "removeRange")]
    pub fn remove_range(&mut self, start: u32, count: u32) {
        let start_idx = start.min(self.len) as usize;
        let end_idx = (start + count).min(self.len) as usize;
        if start_idx < end_idx {
            let byte_start = start_idx * self.element_size;
            let byte_end = end_idx * self.element_size;
            self.elements.drain(byte_start..byte_end);
            self.len -= (end_idx - start_idx) as u32;
        }
    }

    // =========================================================================
    // ITERATOR ACCESS
    // =========================================================================

    /// Iterate over element byte slices.
    pub fn iter_raw(&self) -> impl Iterator<Item = &[u8]> {
        self.elements.chunks(self.element_size).take(self.len as usize)
    }

    /// Iterate over mutable element byte slices.
    pub fn iter_raw_mut(&mut self) -> impl Iterator<Item = &mut [u8]> {
        let len = self.len as usize;
        self.elements.chunks_mut(self.element_size).take(len)
    }
}

impl fmt::Debug for ScriptArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScriptArray")
            .field("element_type_id", &self.element_type_id)
            .field("element_size", &self.element_size)
            .field("len", &self.len)
            .field("ref_count", &self.ref_count.load(AtomicOrdering::Relaxed))
            .finish()
    }
}

// =========================================================================
// MODULE CREATION
// =========================================================================

/// Creates the array module with the `array<T>` template type.
///
/// Registers the built-in array template with:
/// - Reference counting behaviors (addref/release)
/// - Basic size/capacity methods
///
/// # Example
///
/// ```ignore
/// use angelscript_modules::array;
///
/// let module = array::module();
/// // Install with context...
/// ```
pub fn module() -> Module {
    Module::new()
        .class::<ScriptArray>()
}

// =========================================================================
// TESTS
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::primitives;

    #[test]
    fn test_new_creates_empty_array() {
        let arr = ScriptArray::new(primitives::INT32, 4);
        assert!(arr.is_empty());
        assert_eq!(arr.len(), 0);
        assert_eq!(arr.element_type_id(), primitives::INT32);
        assert_eq!(arr.element_size(), 4);
        assert_eq!(arr.ref_count(), 1);
    }

    #[test]
    fn test_with_capacity() {
        let arr = ScriptArray::with_capacity(primitives::INT32, 4, 100);
        assert!(arr.is_empty());
        assert!(arr.capacity() >= 100);
    }

    #[test]
    fn test_ref_count_initial() {
        let arr = ScriptArray::new(primitives::INT32, 4);
        assert_eq!(arr.ref_count(), 1);
    }

    #[test]
    fn test_add_ref() {
        let arr = ScriptArray::new(primitives::INT32, 4);
        arr.add_ref();
        assert_eq!(arr.ref_count(), 2);
        arr.add_ref();
        assert_eq!(arr.ref_count(), 3);
    }

    #[test]
    fn test_release() {
        let arr = ScriptArray::new(primitives::INT32, 4);
        arr.add_ref(); // ref_count = 2
        assert!(!arr.release()); // ref_count = 1, not zero
        assert_eq!(arr.ref_count(), 1);
        assert!(arr.release()); // ref_count = 0, returns true
    }

    #[test]
    fn test_push_and_len() {
        let mut arr = ScriptArray::new(primitives::INT32, 4);
        unsafe {
            arr.push_raw(&42i32.to_ne_bytes());
            arr.push_raw(&100i32.to_ne_bytes());
        }
        assert_eq!(arr.len(), 2);
        assert!(!arr.is_empty());
    }

    #[test]
    fn test_resize() {
        let mut arr = ScriptArray::new(primitives::INT32, 4);
        arr.resize(5);
        assert_eq!(arr.len(), 5);
    }

    #[test]
    fn test_clear() {
        let mut arr = ScriptArray::new(primitives::INT32, 4);
        arr.resize(5);
        arr.clear();
        assert!(arr.is_empty());
        assert_eq!(arr.len(), 0);
    }

    #[test]
    fn test_remove_at() {
        let mut arr = ScriptArray::new(primitives::INT32, 4);
        unsafe {
            arr.push_raw(&1i32.to_ne_bytes());
            arr.push_raw(&2i32.to_ne_bytes());
            arr.push_raw(&3i32.to_ne_bytes());
        }
        arr.remove_at(1);
        assert_eq!(arr.len(), 2);
    }

    #[test]
    fn test_remove_last() {
        let mut arr = ScriptArray::new(primitives::INT32, 4);
        unsafe {
            arr.push_raw(&1i32.to_ne_bytes());
            arr.push_raw(&2i32.to_ne_bytes());
        }
        arr.remove_last();
        assert_eq!(arr.len(), 1);
    }

    #[test]
    fn test_reverse() {
        let mut arr = ScriptArray::new(primitives::INT32, 4);
        unsafe {
            arr.push_raw(&1i32.to_ne_bytes());
            arr.push_raw(&2i32.to_ne_bytes());
            arr.push_raw(&3i32.to_ne_bytes());
        }
        arr.reverse();
        // First element should now be 3
        unsafe {
            let ptr = arr.get_raw(0);
            let value = i32::from_ne_bytes(std::slice::from_raw_parts(ptr, 4).try_into().unwrap());
            assert_eq!(value, 3);
        }
    }

    #[test]
    fn test_module_creates() {
        use angelscript_registry::HasClassMeta;
        let meta = ScriptArray::__as_type_meta();
        assert_eq!(meta.name, "array");
    }

    #[test]
    fn test_debug() {
        let arr = ScriptArray::new(primitives::INT32, 4);
        let debug = format!("{:?}", arr);
        assert!(debug.contains("ScriptArray"));
        assert!(debug.contains("len: 0"));
    }
}
