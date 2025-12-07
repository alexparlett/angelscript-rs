//! ScriptArray - Type-erased, reference-counted array for AngelScript.
//!
//! This is a REFERENCE type - passed by handle with manual reference counting.
//! Elements are stored type-erased as `VmSlot` values.

use std::cmp::Ordering;
use std::fmt;
use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};

use angelscript_ffi::VmSlot;
use angelscript_core::TypeHash;

/// Type-erased array for AngelScript `array<T>` template.
///
/// This is a REFERENCE type with manual reference counting.
/// Elements are stored as `VmSlot` values for type-erased storage.
pub struct ScriptArray {
    /// Type-erased element storage
    elements: Vec<VmSlot>,
    /// Element type for runtime checking
    element_type_id: TypeHash,
    /// Reference count (starts at 1)
    ref_count: AtomicU32,
}

impl ScriptArray {
    // =========================================================================
    // CONSTRUCTORS
    // =========================================================================

    /// Create an empty array for given element type.
    pub fn new(element_type_id: TypeHash) -> Self {
        Self {
            elements: Vec::new(),
            element_type_id,
            ref_count: AtomicU32::new(1),
        }
    }

    /// Create array with initial capacity.
    pub fn with_capacity(element_type_id: TypeHash, capacity: usize) -> Self {
        Self {
            elements: Vec::with_capacity(capacity),
            element_type_id,
            ref_count: AtomicU32::new(1),
        }
    }

    /// Create array with given length, filled with default values.
    pub fn with_length(element_type_id: TypeHash, length: usize) -> Self {
        let mut elements = Vec::with_capacity(length);
        for _ in 0..length {
            elements.push(Self::default_for_type(element_type_id));
        }
        Self {
            elements,
            element_type_id,
            ref_count: AtomicU32::new(1),
        }
    }

    /// Create array filled with a specific value.
    /// Note: The value must be cloneable (not Native variant).
    pub fn filled(element_type_id: TypeHash, length: usize, value: VmSlot) -> Self {
        let mut elements = Vec::with_capacity(length);
        for _ in 0..length {
            if let Some(cloned) = value.clone_if_possible() {
                elements.push(cloned);
            }
        }
        Self {
            elements,
            element_type_id,
            ref_count: AtomicU32::new(1),
        }
    }

    /// Create array from a vector of slots.
    pub fn from_vec(element_type_id: TypeHash, elements: Vec<VmSlot>) -> Self {
        Self {
            elements,
            element_type_id,
            ref_count: AtomicU32::new(1),
        }
    }

    /// Get the element type ID.
    #[inline]
    pub fn element_type_id(&self) -> TypeHash {
        self.element_type_id
    }

    // =========================================================================
    // REFERENCE COUNTING
    // =========================================================================

    /// Increment reference count.
    #[inline]
    pub fn add_ref(&self) {
        self.ref_count.fetch_add(1, AtomicOrdering::Relaxed);
    }

    /// Decrement reference count. Returns true if count reached zero.
    #[inline]
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
    #[inline]
    pub fn len(&self) -> u32 {
        self.elements.len() as u32
    }

    /// Returns true if the array is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Returns the allocated capacity.
    #[inline]
    pub fn capacity(&self) -> u32 {
        self.elements.capacity() as u32
    }

    /// Reserve capacity for at least `additional` more elements.
    #[inline]
    pub fn reserve(&mut self, additional: u32) {
        self.elements.reserve(additional as usize);
    }

    /// Shrink capacity to fit current length.
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.elements.shrink_to_fit();
    }

    /// Remove all elements.
    #[inline]
    pub fn clear(&mut self) {
        self.elements.clear();
    }

    /// Resize array to `new_len` elements with type's default value.
    pub fn resize(&mut self, new_len: u32) {
        let new_len = new_len as usize;
        if new_len > self.elements.len() {
            while self.elements.len() < new_len {
                self.elements.push(Self::default_for_type(self.element_type_id));
            }
        } else {
            self.elements.truncate(new_len);
        }
    }

    /// Resize array with a specific value for new elements.
    /// Note: The value must be cloneable (not Native variant).
    pub fn resize_with(&mut self, new_len: u32, value: VmSlot) {
        let new_len = new_len as usize;
        if new_len > self.elements.len() {
            while self.elements.len() < new_len {
                if let Some(cloned) = value.clone_if_possible() {
                    self.elements.push(cloned);
                }
            }
        } else {
            self.elements.truncate(new_len);
        }
    }

    // =========================================================================
    // ELEMENT ACCESS
    // =========================================================================

    /// Get element at index (bounds-checked).
    #[inline]
    pub fn get(&self, index: u32) -> Option<&VmSlot> {
        self.elements.get(index as usize)
    }

    /// Get mutable element at index (bounds-checked).
    #[inline]
    pub fn get_mut(&mut self, index: u32) -> Option<&mut VmSlot> {
        self.elements.get_mut(index as usize)
    }

    /// Get first element.
    #[inline]
    pub fn first(&self) -> Option<&VmSlot> {
        self.elements.first()
    }

    /// Get mutable first element.
    #[inline]
    pub fn first_mut(&mut self) -> Option<&mut VmSlot> {
        self.elements.first_mut()
    }

    /// Get last element.
    #[inline]
    pub fn last(&self) -> Option<&VmSlot> {
        self.elements.last()
    }

    /// Get mutable last element.
    #[inline]
    pub fn last_mut(&mut self) -> Option<&mut VmSlot> {
        self.elements.last_mut()
    }

    /// Get element at index (unchecked).
    ///
    /// # Safety
    /// Index must be within bounds.
    #[inline]
    pub unsafe fn get_unchecked(&self, index: u32) -> &VmSlot {
        unsafe { self.elements.get_unchecked(index as usize) }
    }

    /// Get mutable element at index (unchecked).
    ///
    /// # Safety
    /// Index must be within bounds.
    #[inline]
    pub unsafe fn get_unchecked_mut(&mut self, index: u32) -> &mut VmSlot {
        unsafe { self.elements.get_unchecked_mut(index as usize) }
    }

    /// Get slice of underlying elements.
    #[inline]
    pub fn as_slice(&self) -> &[VmSlot] {
        &self.elements
    }

    /// Get mutable slice of underlying elements.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [VmSlot] {
        &mut self.elements
    }

    // =========================================================================
    // INSERTION
    // =========================================================================

    /// Append element to end.
    #[inline]
    pub fn push(&mut self, value: VmSlot) {
        self.elements.push(value);
    }

    /// Insert element at position.
    pub fn insert(&mut self, index: u32, value: VmSlot) {
        let index = (index as usize).min(self.elements.len());
        self.elements.insert(index, value);
    }

    /// Append all elements from another array.
    pub fn extend(&mut self, other: &Self) {
        for slot in &other.elements {
            if let Some(cloned) = slot.clone_if_possible() {
                self.elements.push(cloned);
            }
        }
    }

    /// Append elements from a slice.
    pub fn extend_from_slice(&mut self, slice: &[VmSlot]) {
        for slot in slice {
            if let Some(cloned) = slot.clone_if_possible() {
                self.elements.push(cloned);
            }
        }
    }

    // =========================================================================
    // REMOVAL
    // =========================================================================

    /// Remove and return last element.
    #[inline]
    pub fn pop(&mut self) -> Option<VmSlot> {
        self.elements.pop()
    }

    /// Remove element at position and return it.
    pub fn remove_at(&mut self, index: u32) -> Option<VmSlot> {
        let index = index as usize;
        if index < self.elements.len() {
            Some(self.elements.remove(index))
        } else {
            None
        }
    }

    /// Remove range of elements [start..start+count].
    pub fn remove_range(&mut self, start: u32, count: u32) {
        let start = start as usize;
        let end = (start + count as usize).min(self.elements.len());
        if start < self.elements.len() {
            self.elements.drain(start..end);
        }
    }

    /// Keep only elements matching predicate.
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&VmSlot) -> bool,
    {
        self.elements.retain(|slot| f(slot));
    }

    /// Remove consecutive duplicate elements.
    pub fn dedup(&mut self) {
        self.elements.dedup_by(|a, b| Self::slots_equal(a, b));
    }

    // =========================================================================
    // SEARCH
    // =========================================================================

    /// Find first occurrence of value. Returns None if not found.
    pub fn find(&self, value: &VmSlot) -> Option<u32> {
        self.elements
            .iter()
            .position(|slot| Self::slots_equal(slot, value))
            .map(|i| i as u32)
    }

    /// Find first occurrence of value starting from `start`.
    pub fn find_from(&self, start: u32, value: &VmSlot) -> Option<u32> {
        let start = start as usize;
        if start >= self.elements.len() {
            return None;
        }
        self.elements[start..]
            .iter()
            .position(|slot| Self::slots_equal(slot, value))
            .map(|i| (start + i) as u32)
    }

    /// Find last occurrence of value.
    pub fn rfind(&self, value: &VmSlot) -> Option<u32> {
        self.elements
            .iter()
            .rposition(|slot| Self::slots_equal(slot, value))
            .map(|i| i as u32)
    }

    /// Check if array contains value.
    pub fn contains(&self, value: &VmSlot) -> bool {
        self.find(value).is_some()
    }

    /// Count occurrences of value.
    pub fn count(&self, value: &VmSlot) -> u32 {
        self.elements
            .iter()
            .filter(|slot| Self::slots_equal(slot, value))
            .count() as u32
    }

    // =========================================================================
    // ORDERING
    // =========================================================================

    /// Reverse elements in place.
    #[inline]
    pub fn reverse(&mut self) {
        self.elements.reverse();
    }

    /// Sort elements in ascending order.
    pub fn sort_asc(&mut self) {
        self.elements.sort_by(Self::compare_slots);
    }

    /// Sort elements in descending order.
    pub fn sort_desc(&mut self) {
        self.elements.sort_by(|a, b| Self::compare_slots(b, a));
    }

    /// Check if array is sorted in ascending order.
    pub fn is_sorted(&self) -> bool {
        self.elements.windows(2).all(|w| {
            Self::compare_slots(&w[0], &w[1]) != Ordering::Greater
        })
    }

    /// Check if array is sorted in descending order.
    pub fn is_sorted_desc(&self) -> bool {
        self.elements.windows(2).all(|w| {
            Self::compare_slots(&w[0], &w[1]) != Ordering::Less
        })
    }

    // =========================================================================
    // TRANSFORM
    // =========================================================================

    /// Fill all elements with value.
    pub fn fill(&mut self, value: VmSlot) {
        for slot in &mut self.elements {
            if let Some(cloned) = value.clone_if_possible() {
                *slot = cloned;
            }
        }
    }

    /// Swap two elements.
    pub fn swap(&mut self, i: u32, j: u32) {
        let i = i as usize;
        let j = j as usize;
        if i < self.elements.len() && j < self.elements.len() {
            self.elements.swap(i, j);
        }
    }

    /// Rotate elements. Positive rotates right, negative rotates left.
    pub fn rotate(&mut self, amount: i32) {
        if self.elements.is_empty() {
            return;
        }
        let len = self.elements.len();
        let amount = amount.rem_euclid(len as i32) as usize;
        if amount > 0 {
            self.elements.rotate_right(amount);
        }
    }

    // =========================================================================
    // SLICING AND CLONING
    // =========================================================================

    /// Create new array from range [start..end].
    pub fn slice(&self, start: u32, end: u32) -> Self {
        let start = (start as usize).min(self.elements.len());
        let end = (end as usize).min(self.elements.len());
        let elements: Vec<VmSlot> = if start < end {
            self.elements[start..end]
                .iter()
                .filter_map(|s| s.clone_if_possible())
                .collect()
        } else {
            Vec::new()
        };
        Self::from_vec(self.element_type_id, elements)
    }

    /// Create new array from position to end [start..].
    pub fn slice_from(&self, start: u32) -> Self {
        self.slice(start, self.len())
    }

    /// Create new array from start to position [..end].
    pub fn slice_to(&self, end: u32) -> Self {
        self.slice(0, end)
    }

    /// Deep clone the array.
    pub fn clone_array(&self) -> Self {
        let elements: Vec<VmSlot> = self
            .elements
            .iter()
            .filter_map(|s| s.clone_if_possible())
            .collect();
        Self {
            elements,
            element_type_id: self.element_type_id,
            ref_count: AtomicU32::new(1),
        }
    }

    // =========================================================================
    // BINARY SEARCH
    // =========================================================================

    /// Binary search for value in sorted array.
    /// Returns Ok(index) if found, Err(insert_position) if not found.
    pub fn binary_search(&self, value: &VmSlot) -> Result<u32, u32> {
        self.elements
            .binary_search_by(|slot| Self::compare_slots(slot, value))
            .map(|i| i as u32)
            .map_err(|i| i as u32)
    }

    // =========================================================================
    // COMPARISON HELPERS
    // =========================================================================

    /// Compare two VmSlot values for equality.
    pub fn slots_equal(a: &VmSlot, b: &VmSlot) -> bool {
        match (a, b) {
            (VmSlot::Void, VmSlot::Void) => true,
            (VmSlot::Int(a), VmSlot::Int(b)) => a == b,
            (VmSlot::Float(a), VmSlot::Float(b)) => a == b,
            (VmSlot::Bool(a), VmSlot::Bool(b)) => a == b,
            (VmSlot::String(a), VmSlot::String(b)) => a == b,
            (VmSlot::Object(a), VmSlot::Object(b)) => a == b,
            (VmSlot::NullHandle, VmSlot::NullHandle) => true,
            _ => false,
        }
    }

    /// Compare two VmSlot values for ordering.
    pub fn compare_slots(a: &VmSlot, b: &VmSlot) -> Ordering {
        match (a, b) {
            (VmSlot::Int(a), VmSlot::Int(b)) => a.cmp(b),
            (VmSlot::Float(a), VmSlot::Float(b)) => {
                a.partial_cmp(b).unwrap_or(Ordering::Equal)
            }
            (VmSlot::Bool(a), VmSlot::Bool(b)) => a.cmp(b),
            (VmSlot::String(a), VmSlot::String(b)) => a.cmp(b),
            // For other/mismatched types, compare by type index
            _ => Self::slot_type_index(a).cmp(&Self::slot_type_index(b)),
        }
    }

    /// Get a numeric index for slot type (for consistent ordering of different types).
    fn slot_type_index(slot: &VmSlot) -> u8 {
        match slot {
            VmSlot::Void => 0,
            VmSlot::Bool(_) => 1,
            VmSlot::Int(_) => 2,
            VmSlot::Float(_) => 3,
            VmSlot::String(_) => 4,
            VmSlot::Object(_) => 5,
            VmSlot::Native(_) => 6,
            VmSlot::NullHandle => 7,
        }
    }

    /// Get default value for a type.
    pub fn default_for_type(type_id: TypeHash) -> VmSlot {
        use angelscript_core::primitives;

        if type_id == primitives::VOID {
            VmSlot::Void
        } else if type_id == primitives::BOOL {
            VmSlot::Bool(false)
        } else if type_id == primitives::INT8
            || type_id == primitives::INT16
            || type_id == primitives::INT32
            || type_id == primitives::INT64
            || type_id == primitives::UINT8
            || type_id == primitives::UINT16
            || type_id == primitives::UINT32
            || type_id == primitives::UINT64
        {
            VmSlot::Int(0)
        } else if type_id == primitives::FLOAT || type_id == primitives::DOUBLE {
            VmSlot::Float(0.0)
        } else if type_id == primitives::STRING {
            VmSlot::String(String::new())
        } else {
            VmSlot::NullHandle // reference types default to null
        }
    }

    // =========================================================================
    // ITERATOR ACCESS
    // =========================================================================

    /// Iterate over elements.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &VmSlot> {
        self.elements.iter()
    }

    /// Iterate over mutable elements.
    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut VmSlot> {
        self.elements.iter_mut()
    }
}

impl fmt::Debug for ScriptArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScriptArray")
            .field("element_type_id", &self.element_type_id)
            .field("len", &self.elements.len())
            .field("ref_count", &self.ref_count.load(AtomicOrdering::Relaxed))
            .finish()
    }
}

// Note: We don't implement Clone for ScriptArray because it's a reference type.
// Use clone_array() to create a deep copy with ref_count = 1.

// =========================================================================
// FFI REGISTRATION
// =========================================================================

use angelscript_ffi::{CallContext, ListPattern, NativeError, NativeType, TemplateValidation};
use angelscript_module::{RegistrationError, Module};

impl NativeType for ScriptArray {
    const NAME: &'static str = "array";
}

/// Creates the array module with the `array<T>` template type.
///
/// Registers the built-in array template with:
/// - Factory functions
/// - Reference counting behaviors (addref/release)
/// - List factory for initialization lists: `array<int> a = {1, 2, 3}`
/// - Basic size/capacity methods
///
/// # Template
///
/// `array<T>` accepts any element type T.
///
/// # Example
///
/// ```ignore
/// use angelscript::modules::array_module;
///
/// let module = array_module().expect("failed to create array module");
/// // Register with engine...
/// ```
pub fn array_module<'app>() -> Result<Module<'app>, RegistrationError> {
    let mut module = Module::root();

    module
        .register_type::<ScriptArray>("array<class T>")?
        .reference_type()
        // Template validation - arrays can contain any type
        .template_callback(|_info| TemplateValidation::valid())
        // Reference counting
        .addref(ScriptArray::add_ref)
        .release(|arr: &ScriptArray| {
            arr.release();
        })
        // Default factory: array<T>@ f()
        // VM will handle template type instantiation
        .factory_raw("array<T>@ f()", |_ctx: &mut CallContext| {
            // Placeholder: VM handles creating the array with correct element type
            Ok(())
        })?
        // List factory for initialization lists: array<int> a = {1, 2, 3}
        // The pattern uses a placeholder TypeHash(0) since the actual element type
        // comes from template instantiation
        .list_factory(ListPattern::repeat(TypeHash(0)), |ctx: &mut CallContext| {
            // This is a placeholder implementation
            // The VM will need to provide list buffer access
            let _ = ctx;
            Ok(())
        })
        // === Index operators ===
        // T &opIndex(uint index)
        .operator_raw("T &opIndex(uint index)", |ctx: &mut CallContext| {
            let index: u32 = ctx.arg(0)?;
            let arr: &ScriptArray = ctx.this()?;
            if let Some(slot) = arr.get(index) {
                if let Some(cloned) = slot.clone_if_possible() {
                    ctx.set_return_slot(cloned);
                }
            } else {
                return Err(NativeError::other(format!(
                    "Array index {} out of bounds (length {})",
                    index,
                    arr.len()
                )));
            }
            Ok(())
        })?
        // const T &opIndex(uint index) const
        .operator_raw("const T &opIndex(uint index) const", |ctx: &mut CallContext| {
            let index: u32 = ctx.arg(0)?;
            let arr: &ScriptArray = ctx.this()?;
            if let Some(slot) = arr.get(index) {
                if let Some(cloned) = slot.clone_if_possible() {
                    ctx.set_return_slot(cloned);
                }
            } else {
                return Err(NativeError::other(format!(
                    "Array index {} out of bounds (length {})",
                    index,
                    arr.len()
                )));
            }
            Ok(())
        })?
        // === Assignment operator ===
        // array<T> &opAssign(const array<T>&in)
        .operator_raw("array<T> &opAssign(const array<T> &in)", |ctx: &mut CallContext| {
            // For now, a placeholder - the VM will need to handle deep copy
            let _ = ctx;
            Ok(())
        })?
        // === Size methods ===
        .method_raw("uint length() const", |ctx: &mut CallContext| {
            let arr: &ScriptArray = ctx.this()?;
            ctx.set_return(arr.len())?;
            Ok(())
        })?
        .method_raw("bool isEmpty() const", |ctx: &mut CallContext| {
            let arr: &ScriptArray = ctx.this()?;
            ctx.set_return(arr.is_empty())?;
            Ok(())
        })?
        .method_raw("uint capacity() const", |ctx: &mut CallContext| {
            let arr: &ScriptArray = ctx.this()?;
            ctx.set_return(arr.capacity())?;
            Ok(())
        })?
        .method_raw("void clear()", |ctx: &mut CallContext| {
            let arr: &mut ScriptArray = ctx.this_mut()?;
            arr.clear();
            Ok(())
        })?
        .method_raw("void resize(uint length)", |ctx: &mut CallContext| {
            let length: u32 = ctx.arg(0)?;
            let arr: &mut ScriptArray = ctx.this_mut()?;
            arr.resize(length);
            Ok(())
        })?
        .method_raw("void reserve(uint length)", |ctx: &mut CallContext| {
            let length: u32 = ctx.arg(0)?;
            let arr: &mut ScriptArray = ctx.this_mut()?;
            arr.reserve(length);
            Ok(())
        })?
        .method_raw("void shrinkToFit()", |ctx: &mut CallContext| {
            let arr: &mut ScriptArray = ctx.this_mut()?;
            arr.shrink_to_fit();
            Ok(())
        })?
        // === Insertion methods ===
        .method_raw("void insertAt(uint index, const T &in value)", |ctx: &mut CallContext| {
            let index: u32 = ctx.arg(0)?;
            let value = ctx.arg_slot(1)?.clone_if_possible().unwrap_or(VmSlot::Void);
            let arr: &mut ScriptArray = ctx.this_mut()?;
            arr.insert(index, value);
            Ok(())
        })?
        .method_raw("void insertAt(uint index, const array<T> &in arr)", |ctx: &mut CallContext| {
            let index: u32 = ctx.arg(0)?;
            // For inserting another array - placeholder for now
            let _ = index;
            let _ = ctx;
            Ok(())
        })?
        .method_raw("void insertLast(const T &in value)", |ctx: &mut CallContext| {
            let value = ctx.arg_slot(0)?.clone_if_possible().unwrap_or(VmSlot::Void);
            let arr: &mut ScriptArray = ctx.this_mut()?;
            arr.push(value);
            Ok(())
        })?
        // === Removal methods ===
        .method_raw("void removeAt(uint index)", |ctx: &mut CallContext| {
            let index: u32 = ctx.arg(0)?;
            let arr: &mut ScriptArray = ctx.this_mut()?;
            arr.remove_at(index);
            Ok(())
        })?
        .method_raw("void removeLast()", |ctx: &mut CallContext| {
            let arr: &mut ScriptArray = ctx.this_mut()?;
            arr.pop();
            Ok(())
        })?
        .method_raw("void removeRange(uint start, uint count)", |ctx: &mut CallContext| {
            let start: u32 = ctx.arg(0)?;
            let count: u32 = ctx.arg(1)?;
            let arr: &mut ScriptArray = ctx.this_mut()?;
            arr.remove_range(start, count);
            Ok(())
        })?
        // === Ordering methods ===
        .method_raw("void reverse()", |ctx: &mut CallContext| {
            let arr: &mut ScriptArray = ctx.this_mut()?;
            arr.reverse();
            Ok(())
        })?
        .method_raw("void sortAsc()", |ctx: &mut CallContext| {
            let arr: &mut ScriptArray = ctx.this_mut()?;
            arr.sort_asc();
            Ok(())
        })?
        .method_raw("void sortDesc()", |ctx: &mut CallContext| {
            let arr: &mut ScriptArray = ctx.this_mut()?;
            arr.sort_desc();
            Ok(())
        })?
        // TODO: void sortAsc(uint startAt, uint count)
        // TODO: void sortDesc(uint startAt, uint count)
        // TODO: void sort(const less &in compareFunc, uint startAt = 0, uint count = uint(-1))
        // === Search methods ===
        .method_raw("int find(const T &in value) const", |ctx: &mut CallContext| {
            let value = ctx.arg_slot(0)?;
            let arr: &ScriptArray = ctx.this()?;
            let result = arr.find(value).map(|i| i as i64).unwrap_or(-1);
            ctx.set_return(result)?;
            Ok(())
        })?
        .method_raw("int find(uint startAt, const T &in value) const", |ctx: &mut CallContext| {
            let start_at: u32 = ctx.arg(0)?;
            let value = ctx.arg_slot(1)?;
            let arr: &ScriptArray = ctx.this()?;
            let result = arr.find_from(start_at, value).map(|i| i as i64).unwrap_or(-1);
            ctx.set_return(result)?;
            Ok(())
        })?
        // findByRef - same implementation as find for now since we compare by value
        .method_raw("int findByRef(const T &in value) const", |ctx: &mut CallContext| {
            let value = ctx.arg_slot(0)?;
            let arr: &ScriptArray = ctx.this()?;
            let result = arr.find(value).map(|i| i as i64).unwrap_or(-1);
            ctx.set_return(result)?;
            Ok(())
        })?
        .method_raw("int findByRef(uint startAt, const T &in value) const", |ctx: &mut CallContext| {
            let start_at: u32 = ctx.arg(0)?;
            let value = ctx.arg_slot(1)?;
            let arr: &ScriptArray = ctx.this()?;
            let result = arr.find_from(start_at, value).map(|i| i as i64).unwrap_or(-1);
            ctx.set_return(result)?;
            Ok(())
        })?
        .method_raw("bool contains(const T &in value) const", |ctx: &mut CallContext| {
            let value = ctx.arg_slot(0)?;
            let arr: &ScriptArray = ctx.this()?;
            ctx.set_return(arr.contains(value))?;
            Ok(())
        })?
        // === Comparison ===
        .operator_raw("bool opEquals(const array<T> &in) const", |ctx: &mut CallContext| {
            // Placeholder - needs proper implementation comparing element by element
            let _ = ctx;
            ctx.set_return(false)?;
            Ok(())
        })?
        .build()?;

    Ok(module)
}

// =========================================================================
// TESTS
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::primitives;

    // =========================================================================
    // CONSTRUCTOR TESTS
    // =========================================================================

    #[test]
    fn test_new_creates_empty_array() {
        let arr = ScriptArray::new(primitives::INT32);
        assert!(arr.is_empty());
        assert_eq!(arr.len(), 0);
        assert_eq!(arr.element_type_id(), primitives::INT32);
        assert_eq!(arr.ref_count(), 1);
    }

    #[test]
    fn test_with_capacity() {
        let arr = ScriptArray::with_capacity(primitives::INT32, 100);
        assert!(arr.is_empty());
        assert!(arr.capacity() >= 100);
    }

    #[test]
    fn test_with_length() {
        let arr = ScriptArray::with_length(primitives::INT32, 5);
        assert_eq!(arr.len(), 5);
        // All elements should be default (0 for int)
        for i in 0..5 {
            assert!(matches!(arr.get(i), Some(VmSlot::Int(0))));
        }
    }

    #[test]
    fn test_filled() {
        let arr = ScriptArray::filled(primitives::INT32, 3, VmSlot::Int(42));
        assert_eq!(arr.len(), 3);
        for i in 0..3 {
            assert!(matches!(arr.get(i), Some(VmSlot::Int(42))));
        }
    }

    #[test]
    fn test_from_vec() {
        let vec = vec![VmSlot::Int(1), VmSlot::Int(2), VmSlot::Int(3)];
        let arr = ScriptArray::from_vec(primitives::INT32, vec);
        assert_eq!(arr.len(), 3);
        assert!(matches!(arr.get(0), Some(VmSlot::Int(1))));
        assert!(matches!(arr.get(2), Some(VmSlot::Int(3))));
    }

    // =========================================================================
    // REFERENCE COUNTING TESTS
    // =========================================================================

    #[test]
    fn test_ref_count_initial() {
        let arr = ScriptArray::new(primitives::INT32);
        assert_eq!(arr.ref_count(), 1);
    }

    #[test]
    fn test_add_ref() {
        let arr = ScriptArray::new(primitives::INT32);
        arr.add_ref();
        assert_eq!(arr.ref_count(), 2);
        arr.add_ref();
        assert_eq!(arr.ref_count(), 3);
    }

    #[test]
    fn test_release() {
        let arr = ScriptArray::new(primitives::INT32);
        arr.add_ref(); // ref_count = 2
        assert!(!arr.release()); // ref_count = 1, not zero
        assert_eq!(arr.ref_count(), 1);
        assert!(arr.release()); // ref_count = 0, returns true
    }

    #[test]
    fn test_ref_count_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        // Wrap in Arc for sharing across threads
        let arr = Arc::new(ScriptArray::new(primitives::INT32));
        let mut handles = vec![];

        // Spawn threads that add refs
        for _ in 0..10 {
            let arr_clone = Arc::clone(&arr);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    arr_clone.add_ref();
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Initial 1 + 10 threads * 100 add_refs = 1001
        assert_eq!(arr.ref_count(), 1001);
    }

    // =========================================================================
    // SIZE AND CAPACITY TESTS
    // =========================================================================

    #[test]
    fn test_len_and_is_empty() {
        let mut arr = ScriptArray::new(primitives::INT32);
        assert!(arr.is_empty());
        assert_eq!(arr.len(), 0);

        arr.push(VmSlot::Int(1));
        assert!(!arr.is_empty());
        assert_eq!(arr.len(), 1);
    }

    #[test]
    fn test_reserve() {
        let mut arr = ScriptArray::new(primitives::INT32);
        arr.reserve(100);
        assert!(arr.capacity() >= 100);
    }

    #[test]
    fn test_shrink_to_fit() {
        let mut arr = ScriptArray::new(primitives::INT32);
        arr.reserve(1000);
        arr.push(VmSlot::Int(1));
        arr.shrink_to_fit();
        // Capacity should be reduced (exact amount is implementation-defined)
        assert!(arr.capacity() < 1000);
    }

    #[test]
    fn test_clear() {
        let mut arr = ScriptArray::new(primitives::INT32);
        arr.push(VmSlot::Int(1));
        arr.push(VmSlot::Int(2));
        arr.clear();
        assert!(arr.is_empty());
    }

    #[test]
    fn test_resize_grow() {
        let mut arr = ScriptArray::new(primitives::INT32);
        arr.push(VmSlot::Int(1));
        arr.resize(5);
        assert_eq!(arr.len(), 5);
        assert!(matches!(arr.get(0), Some(VmSlot::Int(1))));
        assert!(matches!(arr.get(4), Some(VmSlot::Int(0)))); // default
    }

    #[test]
    fn test_resize_shrink() {
        let mut arr = ScriptArray::from_vec(primitives::INT32, vec![
            VmSlot::Int(1), VmSlot::Int(2), VmSlot::Int(3), VmSlot::Int(4), VmSlot::Int(5)
        ]);
        arr.resize(2);
        assert_eq!(arr.len(), 2);
        assert!(matches!(arr.get(1), Some(VmSlot::Int(2))));
        assert!(arr.get(2).is_none());
    }

    #[test]
    fn test_resize_with() {
        let mut arr = ScriptArray::new(primitives::INT32);
        arr.resize_with(3, VmSlot::Int(99));
        assert_eq!(arr.len(), 3);
        for i in 0..3 {
            assert!(matches!(arr.get(i), Some(VmSlot::Int(99))));
        }
    }

    // =========================================================================
    // ELEMENT ACCESS TESTS
    // =========================================================================

    #[test]
    fn test_get_and_get_mut() {
        let mut arr = ScriptArray::from_vec(primitives::INT32, vec![VmSlot::Int(10), VmSlot::Int(20)]);

        assert!(matches!(arr.get(0), Some(VmSlot::Int(10))));
        assert!(matches!(arr.get(1), Some(VmSlot::Int(20))));
        assert!(arr.get(2).is_none());

        if let Some(VmSlot::Int(v)) = arr.get_mut(0) {
            *v = 100;
        }
        assert!(matches!(arr.get(0), Some(VmSlot::Int(100))));
    }

    #[test]
    fn test_first_and_last() {
        let mut arr = ScriptArray::from_vec(primitives::INT32, vec![
            VmSlot::Int(1), VmSlot::Int(2), VmSlot::Int(3)
        ]);

        assert!(matches!(arr.first(), Some(VmSlot::Int(1))));
        assert!(matches!(arr.last(), Some(VmSlot::Int(3))));

        if let Some(VmSlot::Int(v)) = arr.first_mut() {
            *v = 10;
        }
        if let Some(VmSlot::Int(v)) = arr.last_mut() {
            *v = 30;
        }
        assert!(matches!(arr.first(), Some(VmSlot::Int(10))));
        assert!(matches!(arr.last(), Some(VmSlot::Int(30))));
    }

    #[test]
    fn test_first_last_empty() {
        let arr = ScriptArray::new(primitives::INT32);
        assert!(arr.first().is_none());
        assert!(arr.last().is_none());
    }

    #[test]
    fn test_as_slice() {
        let arr = ScriptArray::from_vec(primitives::INT32, vec![VmSlot::Int(1), VmSlot::Int(2)]);
        let slice = arr.as_slice();
        assert_eq!(slice.len(), 2);
    }

    // =========================================================================
    // INSERTION TESTS
    // =========================================================================

    #[test]
    fn test_push() {
        let mut arr = ScriptArray::new(primitives::INT32);
        arr.push(VmSlot::Int(1));
        arr.push(VmSlot::Int(2));
        arr.push(VmSlot::Int(3));
        assert_eq!(arr.len(), 3);
        assert!(matches!(arr.get(2), Some(VmSlot::Int(3))));
    }

    #[test]
    fn test_insert_middle() {
        let mut arr = ScriptArray::from_vec(primitives::INT32, vec![VmSlot::Int(1), VmSlot::Int(3)]);
        arr.insert(1, VmSlot::Int(2));
        assert_eq!(arr.len(), 3);
        assert!(matches!(arr.get(0), Some(VmSlot::Int(1))));
        assert!(matches!(arr.get(1), Some(VmSlot::Int(2))));
        assert!(matches!(arr.get(2), Some(VmSlot::Int(3))));
    }

    #[test]
    fn test_insert_start() {
        let mut arr = ScriptArray::from_vec(primitives::INT32, vec![VmSlot::Int(2)]);
        arr.insert(0, VmSlot::Int(1));
        assert!(matches!(arr.get(0), Some(VmSlot::Int(1))));
    }

    #[test]
    fn test_insert_end() {
        let mut arr = ScriptArray::from_vec(primitives::INT32, vec![VmSlot::Int(1)]);
        arr.insert(1, VmSlot::Int(2));
        arr.insert(100, VmSlot::Int(3)); // clamped to end
        assert!(matches!(arr.get(2), Some(VmSlot::Int(3))));
    }

    #[test]
    fn test_extend() {
        let mut arr1 = ScriptArray::from_vec(primitives::INT32, vec![VmSlot::Int(1), VmSlot::Int(2)]);
        let arr2 = ScriptArray::from_vec(primitives::INT32, vec![VmSlot::Int(3), VmSlot::Int(4)]);
        arr1.extend(&arr2);
        assert_eq!(arr1.len(), 4);
        assert!(matches!(arr1.get(3), Some(VmSlot::Int(4))));
    }

    // =========================================================================
    // REMOVAL TESTS
    // =========================================================================

    #[test]
    fn test_pop() {
        let mut arr = ScriptArray::from_vec(primitives::INT32, vec![VmSlot::Int(1), VmSlot::Int(2)]);
        let popped = arr.pop();
        assert!(matches!(popped, Some(VmSlot::Int(2))));
        assert_eq!(arr.len(), 1);
    }

    #[test]
    fn test_pop_empty() {
        let mut arr = ScriptArray::new(primitives::INT32);
        assert!(arr.pop().is_none());
    }

    #[test]
    fn test_remove_at() {
        let mut arr = ScriptArray::from_vec(primitives::INT32, vec![
            VmSlot::Int(1), VmSlot::Int(2), VmSlot::Int(3)
        ]);
        let removed = arr.remove_at(1);
        assert!(matches!(removed, Some(VmSlot::Int(2))));
        assert_eq!(arr.len(), 2);
        assert!(matches!(arr.get(1), Some(VmSlot::Int(3))));
    }

    #[test]
    fn test_remove_at_out_of_bounds() {
        let mut arr = ScriptArray::from_vec(primitives::INT32, vec![VmSlot::Int(1)]);
        assert!(arr.remove_at(100).is_none());
        assert_eq!(arr.len(), 1);
    }

    #[test]
    fn test_remove_range() {
        let mut arr = ScriptArray::from_vec(primitives::INT32, vec![
            VmSlot::Int(1), VmSlot::Int(2), VmSlot::Int(3), VmSlot::Int(4), VmSlot::Int(5)
        ]);
        arr.remove_range(1, 2);
        assert_eq!(arr.len(), 3);
        assert!(matches!(arr.get(0), Some(VmSlot::Int(1))));
        assert!(matches!(arr.get(1), Some(VmSlot::Int(4))));
    }

    #[test]
    fn test_retain() {
        let mut arr = ScriptArray::from_vec(primitives::INT32, vec![
            VmSlot::Int(1), VmSlot::Int(2), VmSlot::Int(3), VmSlot::Int(4)
        ]);
        arr.retain(|slot| {
            if let VmSlot::Int(v) = slot {
                *v % 2 == 0
            } else {
                false
            }
        });
        assert_eq!(arr.len(), 2);
        assert!(matches!(arr.get(0), Some(VmSlot::Int(2))));
        assert!(matches!(arr.get(1), Some(VmSlot::Int(4))));
    }

    #[test]
    fn test_dedup() {
        let mut arr = ScriptArray::from_vec(primitives::INT32, vec![
            VmSlot::Int(1), VmSlot::Int(1), VmSlot::Int(2), VmSlot::Int(2), VmSlot::Int(3)
        ]);
        arr.dedup();
        assert_eq!(arr.len(), 3);
        assert!(matches!(arr.get(0), Some(VmSlot::Int(1))));
        assert!(matches!(arr.get(1), Some(VmSlot::Int(2))));
        assert!(matches!(arr.get(2), Some(VmSlot::Int(3))));
    }

    // =========================================================================
    // SEARCH TESTS
    // =========================================================================

    #[test]
    fn test_find() {
        let arr = ScriptArray::from_vec(primitives::INT32, vec![
            VmSlot::Int(10), VmSlot::Int(20), VmSlot::Int(30)
        ]);
        assert_eq!(arr.find(&VmSlot::Int(20)), Some(1));
        assert_eq!(arr.find(&VmSlot::Int(99)), None);
    }

    #[test]
    fn test_find_from() {
        let arr = ScriptArray::from_vec(primitives::INT32, vec![
            VmSlot::Int(1), VmSlot::Int(2), VmSlot::Int(1), VmSlot::Int(2)
        ]);
        assert_eq!(arr.find_from(0, &VmSlot::Int(2)), Some(1));
        assert_eq!(arr.find_from(2, &VmSlot::Int(2)), Some(3));
        assert_eq!(arr.find_from(100, &VmSlot::Int(2)), None);
    }

    #[test]
    fn test_rfind() {
        let arr = ScriptArray::from_vec(primitives::INT32, vec![
            VmSlot::Int(1), VmSlot::Int(2), VmSlot::Int(1), VmSlot::Int(2)
        ]);
        assert_eq!(arr.rfind(&VmSlot::Int(1)), Some(2));
        assert_eq!(arr.rfind(&VmSlot::Int(99)), None);
    }

    #[test]
    fn test_contains() {
        let arr = ScriptArray::from_vec(primitives::INT32, vec![VmSlot::Int(1), VmSlot::Int(2)]);
        assert!(arr.contains(&VmSlot::Int(1)));
        assert!(!arr.contains(&VmSlot::Int(99)));
    }

    #[test]
    fn test_count() {
        let arr = ScriptArray::from_vec(primitives::INT32, vec![
            VmSlot::Int(1), VmSlot::Int(2), VmSlot::Int(1), VmSlot::Int(1)
        ]);
        assert_eq!(arr.count(&VmSlot::Int(1)), 3);
        assert_eq!(arr.count(&VmSlot::Int(2)), 1);
        assert_eq!(arr.count(&VmSlot::Int(99)), 0);
    }

    // =========================================================================
    // ORDERING TESTS
    // =========================================================================

    #[test]
    fn test_reverse() {
        let mut arr = ScriptArray::from_vec(primitives::INT32, vec![
            VmSlot::Int(1), VmSlot::Int(2), VmSlot::Int(3)
        ]);
        arr.reverse();
        assert!(matches!(arr.get(0), Some(VmSlot::Int(3))));
        assert!(matches!(arr.get(1), Some(VmSlot::Int(2))));
        assert!(matches!(arr.get(2), Some(VmSlot::Int(1))));
    }

    #[test]
    fn test_sort_asc() {
        let mut arr = ScriptArray::from_vec(primitives::INT32, vec![
            VmSlot::Int(3), VmSlot::Int(1), VmSlot::Int(2)
        ]);
        arr.sort_asc();
        assert!(matches!(arr.get(0), Some(VmSlot::Int(1))));
        assert!(matches!(arr.get(1), Some(VmSlot::Int(2))));
        assert!(matches!(arr.get(2), Some(VmSlot::Int(3))));
    }

    #[test]
    fn test_sort_desc() {
        let mut arr = ScriptArray::from_vec(primitives::INT32, vec![
            VmSlot::Int(1), VmSlot::Int(3), VmSlot::Int(2)
        ]);
        arr.sort_desc();
        assert!(matches!(arr.get(0), Some(VmSlot::Int(3))));
        assert!(matches!(arr.get(1), Some(VmSlot::Int(2))));
        assert!(matches!(arr.get(2), Some(VmSlot::Int(1))));
    }

    #[test]
    fn test_sort_strings() {
        let mut arr = ScriptArray::from_vec(primitives::STRING, vec![
            VmSlot::String("banana".into()),
            VmSlot::String("apple".into()),
            VmSlot::String("cherry".into()),
        ]);
        arr.sort_asc();
        assert!(matches!(arr.get(0), Some(VmSlot::String(s)) if s == "apple"));
        assert!(matches!(arr.get(1), Some(VmSlot::String(s)) if s == "banana"));
        assert!(matches!(arr.get(2), Some(VmSlot::String(s)) if s == "cherry"));
    }

    #[test]
    fn test_is_sorted() {
        let sorted = ScriptArray::from_vec(primitives::INT32, vec![
            VmSlot::Int(1), VmSlot::Int(2), VmSlot::Int(3)
        ]);
        assert!(sorted.is_sorted());

        let unsorted = ScriptArray::from_vec(primitives::INT32, vec![
            VmSlot::Int(3), VmSlot::Int(1), VmSlot::Int(2)
        ]);
        assert!(!unsorted.is_sorted());
    }

    #[test]
    fn test_is_sorted_desc() {
        let sorted = ScriptArray::from_vec(primitives::INT32, vec![
            VmSlot::Int(3), VmSlot::Int(2), VmSlot::Int(1)
        ]);
        assert!(sorted.is_sorted_desc());

        let unsorted = ScriptArray::from_vec(primitives::INT32, vec![
            VmSlot::Int(1), VmSlot::Int(3), VmSlot::Int(2)
        ]);
        assert!(!unsorted.is_sorted_desc());
    }

    #[test]
    fn test_is_sorted_empty_and_single() {
        let empty = ScriptArray::new(primitives::INT32);
        assert!(empty.is_sorted());
        assert!(empty.is_sorted_desc());

        let single = ScriptArray::from_vec(primitives::INT32, vec![VmSlot::Int(1)]);
        assert!(single.is_sorted());
        assert!(single.is_sorted_desc());
    }

    // =========================================================================
    // TRANSFORM TESTS
    // =========================================================================

    #[test]
    fn test_fill() {
        let mut arr = ScriptArray::from_vec(primitives::INT32, vec![
            VmSlot::Int(1), VmSlot::Int(2), VmSlot::Int(3)
        ]);
        arr.fill(VmSlot::Int(99));
        for i in 0..3 {
            assert!(matches!(arr.get(i), Some(VmSlot::Int(99))));
        }
    }

    #[test]
    fn test_swap() {
        let mut arr = ScriptArray::from_vec(primitives::INT32, vec![
            VmSlot::Int(1), VmSlot::Int(2), VmSlot::Int(3)
        ]);
        arr.swap(0, 2);
        assert!(matches!(arr.get(0), Some(VmSlot::Int(3))));
        assert!(matches!(arr.get(2), Some(VmSlot::Int(1))));
    }

    #[test]
    fn test_swap_out_of_bounds() {
        let mut arr = ScriptArray::from_vec(primitives::INT32, vec![VmSlot::Int(1), VmSlot::Int(2)]);
        arr.swap(0, 100); // Should do nothing
        assert!(matches!(arr.get(0), Some(VmSlot::Int(1))));
    }

    #[test]
    fn test_rotate_right() {
        let mut arr = ScriptArray::from_vec(primitives::INT32, vec![
            VmSlot::Int(1), VmSlot::Int(2), VmSlot::Int(3), VmSlot::Int(4)
        ]);
        arr.rotate(1); // Rotate right by 1
        assert!(matches!(arr.get(0), Some(VmSlot::Int(4))));
        assert!(matches!(arr.get(1), Some(VmSlot::Int(1))));
    }

    #[test]
    fn test_rotate_left() {
        let mut arr = ScriptArray::from_vec(primitives::INT32, vec![
            VmSlot::Int(1), VmSlot::Int(2), VmSlot::Int(3), VmSlot::Int(4)
        ]);
        arr.rotate(-1); // Rotate left by 1
        assert!(matches!(arr.get(0), Some(VmSlot::Int(2))));
        assert!(matches!(arr.get(3), Some(VmSlot::Int(1))));
    }

    #[test]
    fn test_rotate_empty() {
        let mut arr = ScriptArray::new(primitives::INT32);
        arr.rotate(5); // Should do nothing
        assert!(arr.is_empty());
    }

    // =========================================================================
    // SLICING AND CLONING TESTS
    // =========================================================================

    #[test]
    fn test_slice() {
        let arr = ScriptArray::from_vec(primitives::INT32, vec![
            VmSlot::Int(1), VmSlot::Int(2), VmSlot::Int(3), VmSlot::Int(4), VmSlot::Int(5)
        ]);
        let sliced = arr.slice(1, 4);
        assert_eq!(sliced.len(), 3);
        assert!(matches!(sliced.get(0), Some(VmSlot::Int(2))));
        assert!(matches!(sliced.get(2), Some(VmSlot::Int(4))));
        assert_eq!(sliced.ref_count(), 1); // New array has ref_count 1
    }

    #[test]
    fn test_slice_clamped() {
        let arr = ScriptArray::from_vec(primitives::INT32, vec![VmSlot::Int(1), VmSlot::Int(2)]);
        let sliced = arr.slice(0, 100);
        assert_eq!(sliced.len(), 2);
    }

    #[test]
    fn test_slice_empty() {
        let arr = ScriptArray::from_vec(primitives::INT32, vec![VmSlot::Int(1)]);
        let sliced = arr.slice(5, 10); // Out of bounds
        assert!(sliced.is_empty());
    }

    #[test]
    fn test_slice_from() {
        let arr = ScriptArray::from_vec(primitives::INT32, vec![
            VmSlot::Int(1), VmSlot::Int(2), VmSlot::Int(3)
        ]);
        let sliced = arr.slice_from(1);
        assert_eq!(sliced.len(), 2);
        assert!(matches!(sliced.get(0), Some(VmSlot::Int(2))));
    }

    #[test]
    fn test_slice_to() {
        let arr = ScriptArray::from_vec(primitives::INT32, vec![
            VmSlot::Int(1), VmSlot::Int(2), VmSlot::Int(3)
        ]);
        let sliced = arr.slice_to(2);
        assert_eq!(sliced.len(), 2);
        assert!(matches!(sliced.get(1), Some(VmSlot::Int(2))));
    }

    #[test]
    fn test_clone_array() {
        let arr = ScriptArray::from_vec(primitives::INT32, vec![
            VmSlot::Int(1), VmSlot::Int(2), VmSlot::Int(3)
        ]);
        let cloned = arr.clone_array();

        assert_eq!(cloned.len(), arr.len());
        assert_eq!(cloned.element_type_id(), arr.element_type_id());
        assert_eq!(cloned.ref_count(), 1); // Fresh ref count

        // Elements should be equal
        for i in 0..3 {
            assert!(ScriptArray::slots_equal(
                arr.get(i).unwrap(),
                cloned.get(i).unwrap()
            ));
        }
    }

    // =========================================================================
    // BINARY SEARCH TESTS
    // =========================================================================

    #[test]
    fn test_binary_search_found() {
        let arr = ScriptArray::from_vec(primitives::INT32, vec![
            VmSlot::Int(1), VmSlot::Int(2), VmSlot::Int(3), VmSlot::Int(4), VmSlot::Int(5)
        ]);
        assert_eq!(arr.binary_search(&VmSlot::Int(3)), Ok(2));
        assert_eq!(arr.binary_search(&VmSlot::Int(1)), Ok(0));
        assert_eq!(arr.binary_search(&VmSlot::Int(5)), Ok(4));
    }

    #[test]
    fn test_binary_search_not_found() {
        let arr = ScriptArray::from_vec(primitives::INT32, vec![
            VmSlot::Int(1), VmSlot::Int(3), VmSlot::Int(5)
        ]);
        assert_eq!(arr.binary_search(&VmSlot::Int(2)), Err(1)); // Insert at index 1
        assert_eq!(arr.binary_search(&VmSlot::Int(4)), Err(2)); // Insert at index 2
        assert_eq!(arr.binary_search(&VmSlot::Int(0)), Err(0)); // Insert at start
        assert_eq!(arr.binary_search(&VmSlot::Int(6)), Err(3)); // Insert at end
    }

    // =========================================================================
    // COMPARISON HELPER TESTS
    // =========================================================================

    #[test]
    fn test_slots_equal() {
        assert!(ScriptArray::slots_equal(&VmSlot::Int(1), &VmSlot::Int(1)));
        assert!(!ScriptArray::slots_equal(&VmSlot::Int(1), &VmSlot::Int(2)));
        assert!(ScriptArray::slots_equal(&VmSlot::Float(1.5), &VmSlot::Float(1.5)));
        assert!(ScriptArray::slots_equal(&VmSlot::Bool(true), &VmSlot::Bool(true)));
        assert!(ScriptArray::slots_equal(
            &VmSlot::String("test".into()),
            &VmSlot::String("test".into())
        ));
        assert!(ScriptArray::slots_equal(&VmSlot::Void, &VmSlot::Void));
        assert!(ScriptArray::slots_equal(&VmSlot::NullHandle, &VmSlot::NullHandle));

        // Different types
        assert!(!ScriptArray::slots_equal(&VmSlot::Int(1), &VmSlot::Float(1.0)));
    }

    #[test]
    fn test_compare_slots() {
        assert_eq!(ScriptArray::compare_slots(&VmSlot::Int(1), &VmSlot::Int(2)), Ordering::Less);
        assert_eq!(ScriptArray::compare_slots(&VmSlot::Int(2), &VmSlot::Int(1)), Ordering::Greater);
        assert_eq!(ScriptArray::compare_slots(&VmSlot::Int(1), &VmSlot::Int(1)), Ordering::Equal);

        assert_eq!(
            ScriptArray::compare_slots(&VmSlot::Float(1.0), &VmSlot::Float(2.0)),
            Ordering::Less
        );

        assert_eq!(
            ScriptArray::compare_slots(
                &VmSlot::String("apple".into()),
                &VmSlot::String("banana".into())
            ),
            Ordering::Less
        );
    }

    #[test]
    fn test_default_for_type() {
        assert!(matches!(ScriptArray::default_for_type(primitives::VOID), VmSlot::Void));
        assert!(matches!(ScriptArray::default_for_type(primitives::BOOL), VmSlot::Bool(false)));
        assert!(matches!(ScriptArray::default_for_type(primitives::INT32), VmSlot::Int(0)));
        assert!(matches!(ScriptArray::default_for_type(primitives::FLOAT), VmSlot::Float(f) if f == 0.0));
        assert!(matches!(ScriptArray::default_for_type(primitives::STRING), VmSlot::String(ref s) if s.is_empty()));
        assert!(matches!(ScriptArray::default_for_type(TypeHash(100)), VmSlot::NullHandle)); // Unknown type
    }

    // =========================================================================
    // ITERATOR TESTS
    // =========================================================================

    #[test]
    fn test_iter() {
        let arr = ScriptArray::from_vec(primitives::INT32, vec![
            VmSlot::Int(1), VmSlot::Int(2), VmSlot::Int(3)
        ]);
        let sum: i64 = arr.iter().filter_map(|slot| {
            if let VmSlot::Int(v) = slot { Some(*v) } else { None }
        }).sum();
        assert_eq!(sum, 6);
    }

    #[test]
    fn test_iter_mut() {
        let mut arr = ScriptArray::from_vec(primitives::INT32, vec![
            VmSlot::Int(1), VmSlot::Int(2), VmSlot::Int(3)
        ]);
        for slot in arr.iter_mut() {
            if let VmSlot::Int(v) = slot {
                *v *= 2;
            }
        }
        assert!(matches!(arr.get(0), Some(VmSlot::Int(2))));
        assert!(matches!(arr.get(1), Some(VmSlot::Int(4))));
        assert!(matches!(arr.get(2), Some(VmSlot::Int(6))));
    }

    // =========================================================================
    // DEBUG TESTS
    // =========================================================================

    #[test]
    fn test_debug() {
        let arr = ScriptArray::from_vec(primitives::INT32, vec![VmSlot::Int(1), VmSlot::Int(2)]);
        let debug = format!("{:?}", arr);
        assert!(debug.contains("ScriptArray"));
        assert!(debug.contains("len: 2"));
    }

    // =========================================================================
    // EDGE CASE TESTS
    // =========================================================================

    #[test]
    fn test_operations_on_empty_array() {
        let mut arr = ScriptArray::new(primitives::INT32);

        assert!(arr.pop().is_none());
        assert!(arr.first().is_none());
        assert!(arr.last().is_none());
        assert!(arr.find(&VmSlot::Int(1)).is_none());
        assert!(arr.is_sorted());
        arr.reverse(); // Should not panic
        arr.sort_asc(); // Should not panic
    }

    #[test]
    fn test_single_element_array() {
        let mut arr = ScriptArray::from_vec(primitives::INT32, vec![VmSlot::Int(42)]);

        // First and last should both be 42
        assert!(matches!(arr.first(), Some(VmSlot::Int(42))));
        assert!(matches!(arr.last(), Some(VmSlot::Int(42))));
        assert!(arr.is_sorted());
        assert!(arr.is_sorted_desc());

        arr.reverse();
        assert!(matches!(arr.get(0), Some(VmSlot::Int(42))));
    }

    #[test]
    fn test_float_array_with_nan() {
        let mut arr = ScriptArray::from_vec(primitives::FLOAT, vec![
            VmSlot::Float(1.0),
            VmSlot::Float(f64::NAN),
            VmSlot::Float(2.0),
        ]);
        // Sort should handle NaN gracefully (treated as equal)
        arr.sort_asc();
        // Just verify it doesn't panic
        assert_eq!(arr.len(), 3);
    }

    #[test]
    fn test_mixed_type_comparison() {
        // When comparing different types, comparison should be by discriminant
        let int_slot = VmSlot::Int(1);
        let float_slot = VmSlot::Float(1.0);

        // They should not be equal
        assert!(!ScriptArray::slots_equal(&int_slot, &float_slot));

        // But comparison should be consistent
        let cmp1 = ScriptArray::compare_slots(&int_slot, &float_slot);
        let cmp2 = ScriptArray::compare_slots(&float_slot, &int_slot);
        assert_eq!(cmp1.reverse(), cmp2);
    }

    // =========================================================================
    // FFI MODULE TESTS
    // =========================================================================

    #[test]
    fn test_array_module_creates_successfully() {
        let result = array_module();
        assert!(result.is_ok(), "array module should be created successfully");
    }

    #[test]
    fn test_array_module_is_root_namespace() {
        let module = array_module().expect("array module should build");
        assert!(module.is_root(), "array module should be in root namespace");
    }

    #[test]
    fn test_array_module_has_template() {
        let module = array_module().expect("array module should build");
        let types = module.types();
        assert_eq!(types.len(), 1, "should have exactly one type registered");
        assert_eq!(types[0].name, "array", "type should be named 'array'");
    }

    #[test]
    fn test_array_module_has_methods() {
        let module = array_module().expect("array module should build");
        let ty = &module.types()[0];
        // Should have: length, isEmpty, capacity, clear, resize, reserve,
        // shrinkToFit, reverse, removeLast, removeAt, removeRange
        assert!(
            ty.methods.len() >= 10,
            "array should have at least 10 methods, got {}",
            ty.methods.len()
        );
    }

    #[test]
    fn test_array_module_method_names() {
        let module = array_module().expect("array module should build");
        let ty = &module.types()[0];
        let method_names: Vec<_> = ty.methods.iter().map(|m| m.name.as_str()).collect();

        assert!(method_names.contains(&"length"), "should have length method");
        assert!(method_names.contains(&"isEmpty"), "should have isEmpty method");
        assert!(method_names.contains(&"clear"), "should have clear method");
        assert!(method_names.contains(&"resize"), "should have resize method");
        assert!(method_names.contains(&"reverse"), "should have reverse method");
    }

    #[test]
    fn test_array_module_has_behaviors() {
        let module = array_module().expect("array module should build");
        let ty = &module.types()[0];

        assert!(ty.addref.is_some(), "should have addref behavior");
        assert!(ty.release.is_some(), "should have release behavior");
        assert!(ty.list_factory.is_some(), "should have list_factory behavior");
    }

    #[test]
    fn test_native_type_name() {
        assert_eq!(ScriptArray::NAME, "array");
    }
}
