//! ScriptDict - Type-erased, reference-counted dictionary for AngelScript.
//!
//! This is a REFERENCE type - passed by handle with manual reference counting.
//! Keys must be hashable types (primitives, strings, handles).

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};

use ordered_float::OrderedFloat;

use crate::ffi::{ObjectHandle, VmSlot};
use crate::semantic::TypeId;

use super::array::ScriptArray;

/// Hashable wrapper for dictionary keys.
///
/// Only primitives, strings, and handles are valid keys.
/// Void and Native values cannot be used as keys.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScriptKey {
    /// Integer key (any integer type stored as i64)
    Int(i64),
    /// Floating point key (uses OrderedFloat for hashing)
    Float(OrderedFloat<f64>),
    /// Boolean key
    Bool(bool),
    /// String key
    String(String),
    /// Object handle key (compared by identity)
    Handle(ObjectHandle),
    /// Null handle key
    NullHandle,
}

impl ScriptKey {
    /// Try to create a key from a VmSlot.
    ///
    /// Returns None for non-hashable types (Void, Native).
    pub fn from_slot(slot: &VmSlot) -> Option<Self> {
        match slot {
            VmSlot::Int(v) => Some(ScriptKey::Int(*v)),
            VmSlot::Float(v) => Some(ScriptKey::Float(OrderedFloat(*v))),
            VmSlot::Bool(v) => Some(ScriptKey::Bool(*v)),
            VmSlot::String(v) => Some(ScriptKey::String(v.clone())),
            VmSlot::Object(h) => Some(ScriptKey::Handle(*h)),
            VmSlot::NullHandle => Some(ScriptKey::NullHandle),
            VmSlot::Void | VmSlot::Native(_) => None,
        }
    }

    /// Convert key back to VmSlot.
    pub fn to_slot(&self) -> VmSlot {
        match self {
            ScriptKey::Int(v) => VmSlot::Int(*v),
            ScriptKey::Float(v) => VmSlot::Float(v.0),
            ScriptKey::Bool(v) => VmSlot::Bool(*v),
            ScriptKey::String(v) => VmSlot::String(v.clone()),
            ScriptKey::Handle(h) => VmSlot::Object(*h),
            ScriptKey::NullHandle => VmSlot::NullHandle,
        }
    }

    /// Create an integer key.
    #[inline]
    pub fn int(v: i64) -> Self {
        ScriptKey::Int(v)
    }

    /// Create a float key.
    #[inline]
    pub fn float(v: f64) -> Self {
        ScriptKey::Float(OrderedFloat(v))
    }

    /// Create a boolean key.
    #[inline]
    pub fn bool(v: bool) -> Self {
        ScriptKey::Bool(v)
    }

    /// Create a string key.
    #[inline]
    pub fn string(v: impl Into<String>) -> Self {
        ScriptKey::String(v.into())
    }

    /// Create a handle key.
    #[inline]
    pub fn handle(h: ObjectHandle) -> Self {
        ScriptKey::Handle(h)
    }

    /// Create a null handle key.
    #[inline]
    pub fn null() -> Self {
        ScriptKey::NullHandle
    }
}

/// Type-erased dictionary for AngelScript `dictionary<K,V>` template.
///
/// This is a REFERENCE type with manual reference counting.
/// Keys are stored as `ScriptKey` (hashable) and values as `VmSlot`.
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
    // =========================================================================
    // CONSTRUCTORS
    // =========================================================================

    /// Create an empty dictionary for given key and value types.
    pub fn new(key_type_id: TypeId, value_type_id: TypeId) -> Self {
        Self {
            entries: HashMap::new(),
            key_type_id,
            value_type_id,
            ref_count: AtomicU32::new(1),
        }
    }

    /// Create dictionary with initial capacity.
    pub fn with_capacity(key_type_id: TypeId, value_type_id: TypeId, capacity: usize) -> Self {
        Self {
            entries: HashMap::with_capacity(capacity),
            key_type_id,
            value_type_id,
            ref_count: AtomicU32::new(1),
        }
    }

    /// Get the key type ID.
    #[inline]
    pub fn key_type_id(&self) -> TypeId {
        self.key_type_id
    }

    /// Get the value type ID.
    #[inline]
    pub fn value_type_id(&self) -> TypeId {
        self.value_type_id
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

    /// Returns the number of entries.
    #[inline]
    pub fn len(&self) -> u32 {
        self.entries.len() as u32
    }

    /// Returns true if the dictionary is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the allocated capacity.
    #[inline]
    pub fn capacity(&self) -> u32 {
        self.entries.capacity() as u32
    }

    /// Reserve capacity for at least `additional` more entries.
    #[inline]
    pub fn reserve(&mut self, additional: u32) {
        self.entries.reserve(additional as usize);
    }

    /// Shrink capacity to fit current number of entries.
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.entries.shrink_to_fit();
    }

    // =========================================================================
    // INSERTION AND UPDATE
    // =========================================================================

    /// Insert or update an entry. Returns the old value if key existed.
    pub fn insert(&mut self, key: ScriptKey, value: VmSlot) -> Option<VmSlot> {
        self.entries.insert(key, value)
    }

    /// Get existing value or insert default.
    pub fn get_or_insert(&mut self, key: ScriptKey, default: VmSlot) -> &VmSlot {
        self.entries.entry(key).or_insert(default)
    }

    /// Get existing value or insert with function.
    pub fn get_or_insert_with<F>(&mut self, key: ScriptKey, f: F) -> &VmSlot
    where
        F: FnOnce() -> VmSlot,
    {
        self.entries.entry(key).or_insert_with(f)
    }

    /// Insert only if key is absent. Returns true if inserted.
    pub fn try_insert(&mut self, key: ScriptKey, value: VmSlot) -> bool {
        if let std::collections::hash_map::Entry::Vacant(e) = self.entries.entry(key) {
            e.insert(value);
            true
        } else {
            false
        }
    }

    // =========================================================================
    // RETRIEVAL
    // =========================================================================

    /// Get value by key.
    #[inline]
    pub fn get(&self, key: &ScriptKey) -> Option<&VmSlot> {
        self.entries.get(key)
    }

    /// Get mutable value by key.
    #[inline]
    pub fn get_mut(&mut self, key: &ScriptKey) -> Option<&mut VmSlot> {
        self.entries.get_mut(key)
    }

    /// Get value or return default (cloned).
    pub fn get_or(&self, key: &ScriptKey, default: VmSlot) -> VmSlot {
        self.entries
            .get(key)
            .and_then(|v| v.clone_if_possible())
            .unwrap_or(default)
    }

    /// Check if key exists.
    #[inline]
    pub fn contains_key(&self, key: &ScriptKey) -> bool {
        self.entries.contains_key(key)
    }

    // =========================================================================
    // REMOVAL
    // =========================================================================

    /// Remove entry and return value.
    pub fn remove(&mut self, key: &ScriptKey) -> Option<VmSlot> {
        self.entries.remove(key)
    }

    /// Remove all entries.
    #[inline]
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    // =========================================================================
    // KEY/VALUE ACCESS
    // =========================================================================

    /// Get all keys as a vector.
    pub fn keys(&self) -> Vec<ScriptKey> {
        self.entries.keys().cloned().collect()
    }

    /// Get all keys as a ScriptArray of VmSlots.
    pub fn keys_array(&self) -> ScriptArray {
        let elements: Vec<VmSlot> = self.entries.keys().map(|k| k.to_slot()).collect();
        ScriptArray::from_vec(self.key_type_id, elements)
    }

    /// Get all values as a vector.
    pub fn values(&self) -> Vec<VmSlot> {
        self.entries
            .values()
            .filter_map(|v| v.clone_if_possible())
            .collect()
    }

    /// Get all values as a ScriptArray.
    pub fn values_array(&self) -> ScriptArray {
        let elements: Vec<VmSlot> = self
            .entries
            .values()
            .filter_map(|v| v.clone_if_possible())
            .collect();
        ScriptArray::from_vec(self.value_type_id, elements)
    }

    /// Iterate over key-value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&ScriptKey, &VmSlot)> {
        self.entries.iter()
    }

    /// Iterate over mutable values.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&ScriptKey, &mut VmSlot)> {
        self.entries.iter_mut()
    }

    // =========================================================================
    // BULK OPERATIONS
    // =========================================================================

    /// Insert all entries from another dictionary.
    pub fn extend(&mut self, other: &Self) {
        for (key, value) in &other.entries {
            if let Some(cloned) = value.clone_if_possible() {
                self.entries.insert(key.clone(), cloned);
            }
        }
    }

    /// Keep only entries matching predicate.
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&ScriptKey, &VmSlot) -> bool,
    {
        self.entries.retain(|k, v| f(k, v));
    }

    /// Deep clone the dictionary.
    pub fn clone_dict(&self) -> Self {
        let entries: HashMap<ScriptKey, VmSlot> = self
            .entries
            .iter()
            .filter_map(|(k, v)| v.clone_if_possible().map(|v| (k.clone(), v)))
            .collect();

        Self {
            entries,
            key_type_id: self.key_type_id,
            value_type_id: self.value_type_id,
            ref_count: AtomicU32::new(1),
        }
    }

    // =========================================================================
    // PREDICATES
    // =========================================================================

    /// Check if dictionary contains a specific value (linear search).
    pub fn contains_value(&self, value: &VmSlot) -> bool {
        self.entries
            .values()
            .any(|v| ScriptArray::slots_equal(v, value))
    }

    /// Count occurrences of a value.
    pub fn count_value(&self, value: &VmSlot) -> u32 {
        self.entries
            .values()
            .filter(|v| ScriptArray::slots_equal(v, value))
            .count() as u32
    }
}

impl fmt::Debug for ScriptDict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScriptDict")
            .field("key_type_id", &self.key_type_id)
            .field("value_type_id", &self.value_type_id)
            .field("len", &self.entries.len())
            .field("ref_count", &self.ref_count.load(AtomicOrdering::Relaxed))
            .finish()
    }
}

// =========================================================================
// HELPER FUNCTIONS
// =========================================================================

/// Check if a TypeId represents a hashable type (valid for dictionary keys).
///
/// For now, we allow all types except void as dictionary keys.
/// In the future, this should check the type's traits to verify hashability.
pub fn is_hashable_type(type_id: TypeId) -> bool {
    // Void is not hashable
    type_id.0 != 0
}

// =========================================================================
// FFI REGISTRATION
// =========================================================================

use crate::ffi::{CallContext, ListPattern, NativeType, TemplateInstanceInfo, TemplateValidation};
use crate::module::FfiModuleError;
use crate::Module;

impl NativeType for ScriptDict {
    const NAME: &'static str = "dictionary";
}

/// Creates the dictionary module with the `dictionary<K,V>` template type.
///
/// Registers the built-in dictionary template with:
/// - Reference counting behaviors (addref/release)
/// - Template validation (K must be hashable)
/// - List factory for initialization lists: `dictionary@ d = {{"a", 1}, {"b", 2}}`
/// - Basic size/capacity methods
///
/// # Template
///
/// `dictionary<K,V>` requires K to be a hashable type (primitive, string, or handle).
///
/// # Example
///
/// ```ignore
/// use angelscript::modules::dictionary_module;
///
/// let module = dictionary_module().expect("failed to create dictionary module");
/// // Register with engine...
/// ```
pub fn dictionary_module<'app>() -> Result<Module<'app>, FfiModuleError> {
    let mut module = Module::root();

    module
        .register_type::<ScriptDict>("dictionary<class K, class V>")
        .reference_type()
        // Template validation - K must be hashable
        .template_callback(validate_dictionary_template)
        // Reference counting
        .addref(ScriptDict::add_ref)
        .release(|dict: &ScriptDict| {
            dict.release();
        })
        // List factory for initialization lists
        // Uses RepeatTuple pattern for {K, V} pairs
        .list_factory(
            ListPattern::repeat_tuple(vec![TypeId(0), TypeId(0)]),
            |ctx: &mut CallContext| {
                // Placeholder - VM will provide list buffer access
                let _ = ctx;
                Ok(())
            },
        )
        // Basic methods
        .method_raw("uint getSize() const", |ctx: &mut CallContext| {
            let dict: &ScriptDict = ctx.this()?;
            ctx.set_return(dict.len())?;
            Ok(())
        })?
        .method_raw("bool isEmpty() const", |ctx: &mut CallContext| {
            let dict: &ScriptDict = ctx.this()?;
            ctx.set_return(dict.is_empty())?;
            Ok(())
        })?
        .method_raw("uint capacity() const", |ctx: &mut CallContext| {
            let dict: &ScriptDict = ctx.this()?;
            ctx.set_return(dict.capacity())?;
            Ok(())
        })?
        .method_raw("void reserve(uint additional)", |ctx: &mut CallContext| {
            let additional: u32 = ctx.arg(0)?;
            let dict: &mut ScriptDict = ctx.this_mut()?;
            dict.reserve(additional);
            Ok(())
        })?
        .method_raw("void shrinkToFit()", |ctx: &mut CallContext| {
            let dict: &mut ScriptDict = ctx.this_mut()?;
            dict.shrink_to_fit();
            Ok(())
        })?
        .method_raw("void clear()", |ctx: &mut CallContext| {
            let dict: &mut ScriptDict = ctx.this_mut()?;
            dict.clear();
            Ok(())
        })?
        // === Access methods ===
        .method_raw("void set(const K &in key, const V &in value)", |ctx: &mut CallContext| {
            // Clone values before borrowing dict mutably
            let key_slot = ctx.arg_slot(0)?.clone_if_possible();
            let value_slot = ctx.arg_slot(1)?.clone_if_possible();
            if let (Some(key_slot), Some(value_slot)) = (key_slot, value_slot)
                && let Some(key) = ScriptKey::from_slot(&key_slot) {
                    let dict: &mut ScriptDict = ctx.this_mut()?;
                    dict.insert(key, value_slot);
                }
            Ok(())
        })?
        .method_raw("bool exists(const K &in key) const", |ctx: &mut CallContext| {
            let key_slot = ctx.arg_slot(0)?.clone_if_possible();
            let dict: &ScriptDict = ctx.this()?;
            let result = if let Some(key_slot) = key_slot {
                if let Some(key) = ScriptKey::from_slot(&key_slot) {
                    dict.contains_key(&key)
                } else {
                    false
                }
            } else {
                false
            };
            ctx.set_return(result)?;
            Ok(())
        })?
        // get with output parameter - returns true if found
        .method_raw("bool get(const K &in key, V &out value) const", |ctx: &mut CallContext| {
            let key_slot = ctx.arg_slot(0)?.clone_if_possible();
            let dict: &ScriptDict = ctx.this()?;
            let found = if let Some(key_slot) = key_slot {
                if let Some(key) = ScriptKey::from_slot(&key_slot) {
                    dict.get(&key).and_then(|v| v.clone_if_possible())
                } else {
                    None
                }
            } else {
                None
            };
            if let Some(value) = found {
                *ctx.arg_slot_mut(1)? = value;
                ctx.set_return(true)?;
            } else {
                ctx.set_return(false)?;
            }
            Ok(())
        })?
        .method_raw("bool delete(const K &in key)", |ctx: &mut CallContext| {
            // Clone key before borrowing dict mutably
            let key_slot = ctx.arg_slot(0)?.clone_if_possible();
            let result = if let Some(key_slot) = key_slot {
                if let Some(key) = ScriptKey::from_slot(&key_slot) {
                    let dict: &mut ScriptDict = ctx.this_mut()?;
                    dict.remove(&key).is_some()
                } else {
                    false
                }
            } else {
                false
            };
            ctx.set_return(result)?;
            Ok(())
        })?
        // === Index operators ===
        .operator_raw("V &opIndex(const K &in key)", |ctx: &mut CallContext| {
            // Clone key before borrowing dict
            let key_slot = ctx.arg_slot(0)?.clone_if_possible();
            if let Some(key_slot) = key_slot
                && let Some(key) = ScriptKey::from_slot(&key_slot) {
                    let dict: &mut ScriptDict = ctx.this_mut()?;
                    if let Some(value) = dict.get(&key)
                        && let Some(cloned) = value.clone_if_possible() {
                            ctx.set_return_slot(cloned);
                        }
                }
            Ok(())
        })?
        .operator_raw("const V &opIndex(const K &in key) const", |ctx: &mut CallContext| {
            let key_slot = ctx.arg_slot(0)?.clone_if_possible();
            if let Some(key_slot) = key_slot
                && let Some(key) = ScriptKey::from_slot(&key_slot) {
                    let dict: &ScriptDict = ctx.this()?;
                    if let Some(value) = dict.get(&key)
                        && let Some(cloned) = value.clone_if_possible() {
                            ctx.set_return_slot(cloned);
                        }
                }
            Ok(())
        })?
        .build()?;

    Ok(module)
}

/// Validate that dictionary template has a hashable key type.
fn validate_dictionary_template(info: &TemplateInstanceInfo) -> TemplateValidation {
    // K (first type argument) must be hashable
    if info.sub_types.is_empty() {
        return TemplateValidation::invalid("Dictionary requires two type parameters");
    }

    let key_type = &info.sub_types[0];
    if is_hashable_type(key_type.type_id) {
        TemplateValidation::valid()
    } else {
        TemplateValidation::invalid(
            "Dictionary key must be hashable (primitive, string, or handle)",
        )
    }
}

// =========================================================================
// TESTS
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Type ID constants for tests
    const INT_TYPE: TypeId = TypeId(4);     // int32
    const STRING_TYPE: TypeId = TypeId(16);
    const FLOAT_TYPE: TypeId = TypeId(10);
    const BOOL_TYPE: TypeId = TypeId(1);
    const DOUBLE_TYPE: TypeId = TypeId(11);

    // =========================================================================
    // SCRIPT KEY TESTS
    // =========================================================================

    #[test]
    fn test_script_key_from_slot_int() {
        let slot = VmSlot::Int(42);
        let key = ScriptKey::from_slot(&slot).unwrap();
        assert_eq!(key, ScriptKey::Int(42));
    }

    #[test]
    fn test_script_key_from_slot_float() {
        let slot = VmSlot::Float(3.14);
        let key = ScriptKey::from_slot(&slot).unwrap();
        assert_eq!(key, ScriptKey::Float(OrderedFloat(3.14)));
    }

    #[test]
    fn test_script_key_from_slot_bool() {
        let slot = VmSlot::Bool(true);
        let key = ScriptKey::from_slot(&slot).unwrap();
        assert_eq!(key, ScriptKey::Bool(true));
    }

    #[test]
    fn test_script_key_from_slot_string() {
        let slot = VmSlot::String("hello".into());
        let key = ScriptKey::from_slot(&slot).unwrap();
        assert_eq!(key, ScriptKey::String("hello".into()));
    }

    #[test]
    fn test_script_key_from_slot_null() {
        let slot = VmSlot::NullHandle;
        let key = ScriptKey::from_slot(&slot).unwrap();
        assert_eq!(key, ScriptKey::NullHandle);
    }

    #[test]
    fn test_script_key_from_slot_void_fails() {
        let slot = VmSlot::Void;
        assert!(ScriptKey::from_slot(&slot).is_none());
    }

    #[test]
    fn test_script_key_to_slot() {
        assert!(matches!(ScriptKey::Int(42).to_slot(), VmSlot::Int(42)));
        assert!(matches!(ScriptKey::Bool(true).to_slot(), VmSlot::Bool(true)));
        assert!(matches!(ScriptKey::NullHandle.to_slot(), VmSlot::NullHandle));
    }

    #[test]
    fn test_script_key_constructors() {
        assert_eq!(ScriptKey::int(42), ScriptKey::Int(42));
        assert_eq!(ScriptKey::float(3.14), ScriptKey::Float(OrderedFloat(3.14)));
        assert_eq!(ScriptKey::bool(true), ScriptKey::Bool(true));
        assert_eq!(ScriptKey::string("hello"), ScriptKey::String("hello".into()));
        assert_eq!(ScriptKey::null(), ScriptKey::NullHandle);
    }

    #[test]
    fn test_script_key_hash_equality() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(ScriptKey::Int(1));
        set.insert(ScriptKey::Int(2));
        set.insert(ScriptKey::Int(1)); // Duplicate

        assert_eq!(set.len(), 2);
        assert!(set.contains(&ScriptKey::Int(1)));
    }

    #[test]
    fn test_float_key_nan_handling() {
        use std::collections::HashSet;

        // NaN should be comparable with OrderedFloat
        let mut set = HashSet::new();
        set.insert(ScriptKey::Float(OrderedFloat(f64::NAN)));
        set.insert(ScriptKey::Float(OrderedFloat(f64::NAN)));

        // Both NaNs should be equal with OrderedFloat
        assert_eq!(set.len(), 1);
    }

    // =========================================================================
    // CONSTRUCTOR TESTS
    // =========================================================================

    #[test]
    fn test_new_creates_empty_dict() {
        let dict = ScriptDict::new(STRING_TYPE, INT_TYPE);
        assert!(dict.is_empty());
        assert_eq!(dict.len(), 0);
        assert_eq!(dict.key_type_id(), STRING_TYPE);
        assert_eq!(dict.value_type_id(), INT_TYPE);
        assert_eq!(dict.ref_count(), 1);
    }

    #[test]
    fn test_with_capacity() {
        let dict = ScriptDict::with_capacity(STRING_TYPE, INT_TYPE, 100);
        assert!(dict.is_empty());
        assert!(dict.capacity() >= 100);
    }

    // =========================================================================
    // REFERENCE COUNTING TESTS
    // =========================================================================

    #[test]
    fn test_ref_count_initial() {
        let dict = ScriptDict::new(INT_TYPE, INT_TYPE);
        assert_eq!(dict.ref_count(), 1);
    }

    #[test]
    fn test_add_ref() {
        let dict = ScriptDict::new(INT_TYPE, INT_TYPE);
        dict.add_ref();
        assert_eq!(dict.ref_count(), 2);
        dict.add_ref();
        assert_eq!(dict.ref_count(), 3);
    }

    #[test]
    fn test_release() {
        let dict = ScriptDict::new(INT_TYPE, INT_TYPE);
        dict.add_ref(); // ref_count = 2
        assert!(!dict.release()); // ref_count = 1, not zero
        assert_eq!(dict.ref_count(), 1);
        assert!(dict.release()); // ref_count = 0, returns true
    }

    #[test]
    fn test_ref_count_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let dict = Arc::new(ScriptDict::new(INT_TYPE, INT_TYPE));
        let mut handles = vec![];

        for _ in 0..10 {
            let dict_clone = Arc::clone(&dict);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    dict_clone.add_ref();
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(dict.ref_count(), 1001);
    }

    // =========================================================================
    // SIZE AND CAPACITY TESTS
    // =========================================================================

    #[test]
    fn test_len_and_is_empty() {
        let mut dict = ScriptDict::new(STRING_TYPE, INT_TYPE);
        assert!(dict.is_empty());
        assert_eq!(dict.len(), 0);

        dict.insert(ScriptKey::string("one"), VmSlot::Int(1));
        assert!(!dict.is_empty());
        assert_eq!(dict.len(), 1);
    }

    #[test]
    fn test_reserve() {
        let mut dict = ScriptDict::new(INT_TYPE, INT_TYPE);
        dict.reserve(100);
        assert!(dict.capacity() >= 100);
    }

    #[test]
    fn test_shrink_to_fit() {
        let mut dict = ScriptDict::new(INT_TYPE, INT_TYPE);
        dict.reserve(1000);
        dict.insert(ScriptKey::int(1), VmSlot::Int(1));
        dict.shrink_to_fit();
        // Capacity should be reduced
        assert!(dict.capacity() < 1000);
    }

    // =========================================================================
    // INSERTION AND UPDATE TESTS
    // =========================================================================

    #[test]
    fn test_insert_new() {
        let mut dict = ScriptDict::new(STRING_TYPE, INT_TYPE);
        let old = dict.insert(ScriptKey::string("hello"), VmSlot::Int(42));
        assert!(old.is_none());
        assert_eq!(dict.len(), 1);
    }

    #[test]
    fn test_insert_update() {
        let mut dict = ScriptDict::new(STRING_TYPE, INT_TYPE);
        dict.insert(ScriptKey::string("hello"), VmSlot::Int(42));
        let old = dict.insert(ScriptKey::string("hello"), VmSlot::Int(100));
        assert!(matches!(old, Some(VmSlot::Int(42))));
        assert!(matches!(dict.get(&ScriptKey::string("hello")), Some(VmSlot::Int(100))));
    }

    #[test]
    fn test_get_or_insert_new() {
        let mut dict = ScriptDict::new(STRING_TYPE, INT_TYPE);
        let value = dict.get_or_insert(ScriptKey::string("key"), VmSlot::Int(42));
        assert!(matches!(value, VmSlot::Int(42)));
        assert_eq!(dict.len(), 1);
    }

    #[test]
    fn test_get_or_insert_existing() {
        let mut dict = ScriptDict::new(STRING_TYPE, INT_TYPE);
        dict.insert(ScriptKey::string("key"), VmSlot::Int(1));
        let value = dict.get_or_insert(ScriptKey::string("key"), VmSlot::Int(42));
        assert!(matches!(value, VmSlot::Int(1))); // Original value
    }

    #[test]
    fn test_try_insert_new() {
        let mut dict = ScriptDict::new(STRING_TYPE, INT_TYPE);
        assert!(dict.try_insert(ScriptKey::string("key"), VmSlot::Int(42)));
        assert_eq!(dict.len(), 1);
    }

    #[test]
    fn test_try_insert_existing() {
        let mut dict = ScriptDict::new(STRING_TYPE, INT_TYPE);
        dict.insert(ScriptKey::string("key"), VmSlot::Int(1));
        assert!(!dict.try_insert(ScriptKey::string("key"), VmSlot::Int(42)));
        // Value unchanged
        assert!(matches!(dict.get(&ScriptKey::string("key")), Some(VmSlot::Int(1))));
    }

    // =========================================================================
    // RETRIEVAL TESTS
    // =========================================================================

    #[test]
    fn test_get() {
        let mut dict = ScriptDict::new(STRING_TYPE, INT_TYPE);
        dict.insert(ScriptKey::string("hello"), VmSlot::Int(42));

        assert!(matches!(dict.get(&ScriptKey::string("hello")), Some(VmSlot::Int(42))));
        assert!(dict.get(&ScriptKey::string("missing")).is_none());
    }

    #[test]
    fn test_get_mut() {
        let mut dict = ScriptDict::new(STRING_TYPE, INT_TYPE);
        dict.insert(ScriptKey::string("hello"), VmSlot::Int(42));

        if let Some(VmSlot::Int(v)) = dict.get_mut(&ScriptKey::string("hello")) {
            *v = 100;
        }
        assert!(matches!(dict.get(&ScriptKey::string("hello")), Some(VmSlot::Int(100))));
    }

    #[test]
    fn test_get_or() {
        let mut dict = ScriptDict::new(STRING_TYPE, INT_TYPE);
        dict.insert(ScriptKey::string("hello"), VmSlot::Int(42));

        let value = dict.get_or(&ScriptKey::string("hello"), VmSlot::Int(0));
        assert!(matches!(value, VmSlot::Int(42)));

        let default = dict.get_or(&ScriptKey::string("missing"), VmSlot::Int(99));
        assert!(matches!(default, VmSlot::Int(99)));
    }

    #[test]
    fn test_contains_key() {
        let mut dict = ScriptDict::new(STRING_TYPE, INT_TYPE);
        dict.insert(ScriptKey::string("hello"), VmSlot::Int(42));

        assert!(dict.contains_key(&ScriptKey::string("hello")));
        assert!(!dict.contains_key(&ScriptKey::string("missing")));
    }

    // =========================================================================
    // REMOVAL TESTS
    // =========================================================================

    #[test]
    fn test_remove() {
        let mut dict = ScriptDict::new(STRING_TYPE, INT_TYPE);
        dict.insert(ScriptKey::string("hello"), VmSlot::Int(42));

        let removed = dict.remove(&ScriptKey::string("hello"));
        assert!(matches!(removed, Some(VmSlot::Int(42))));
        assert!(dict.is_empty());
    }

    #[test]
    fn test_remove_missing() {
        let mut dict = ScriptDict::new(STRING_TYPE, INT_TYPE);
        assert!(dict.remove(&ScriptKey::string("missing")).is_none());
    }

    #[test]
    fn test_clear() {
        let mut dict = ScriptDict::new(STRING_TYPE, INT_TYPE);
        dict.insert(ScriptKey::string("one"), VmSlot::Int(1));
        dict.insert(ScriptKey::string("two"), VmSlot::Int(2));
        dict.clear();
        assert!(dict.is_empty());
    }

    // =========================================================================
    // KEY/VALUE ACCESS TESTS
    // =========================================================================

    #[test]
    fn test_keys() {
        let mut dict = ScriptDict::new(INT_TYPE, INT_TYPE);
        dict.insert(ScriptKey::int(1), VmSlot::Int(10));
        dict.insert(ScriptKey::int(2), VmSlot::Int(20));

        let keys = dict.keys();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&ScriptKey::int(1)));
        assert!(keys.contains(&ScriptKey::int(2)));
    }

    #[test]
    fn test_keys_array() {
        let mut dict = ScriptDict::new(INT_TYPE, INT_TYPE);
        dict.insert(ScriptKey::int(1), VmSlot::Int(10));
        dict.insert(ScriptKey::int(2), VmSlot::Int(20));

        let keys_arr = dict.keys_array();
        assert_eq!(keys_arr.len(), 2);
    }

    #[test]
    fn test_values() {
        let mut dict = ScriptDict::new(INT_TYPE, INT_TYPE);
        dict.insert(ScriptKey::int(1), VmSlot::Int(10));
        dict.insert(ScriptKey::int(2), VmSlot::Int(20));

        let values = dict.values();
        assert_eq!(values.len(), 2);
    }

    #[test]
    fn test_values_array() {
        let mut dict = ScriptDict::new(INT_TYPE, INT_TYPE);
        dict.insert(ScriptKey::int(1), VmSlot::Int(10));
        dict.insert(ScriptKey::int(2), VmSlot::Int(20));

        let values_arr = dict.values_array();
        assert_eq!(values_arr.len(), 2);
    }

    #[test]
    fn test_iter() {
        let mut dict = ScriptDict::new(INT_TYPE, INT_TYPE);
        dict.insert(ScriptKey::int(1), VmSlot::Int(10));
        dict.insert(ScriptKey::int(2), VmSlot::Int(20));

        let sum: i64 = dict
            .iter()
            .filter_map(|(_, v)| if let VmSlot::Int(n) = v { Some(*n) } else { None })
            .sum();
        assert_eq!(sum, 30);
    }

    // =========================================================================
    // BULK OPERATION TESTS
    // =========================================================================

    #[test]
    fn test_extend() {
        let mut dict1 = ScriptDict::new(INT_TYPE, INT_TYPE);
        dict1.insert(ScriptKey::int(1), VmSlot::Int(10));

        let mut dict2 = ScriptDict::new(INT_TYPE, INT_TYPE);
        dict2.insert(ScriptKey::int(2), VmSlot::Int(20));
        dict2.insert(ScriptKey::int(3), VmSlot::Int(30));

        dict1.extend(&dict2);
        assert_eq!(dict1.len(), 3);
    }

    #[test]
    fn test_retain() {
        let mut dict = ScriptDict::new(INT_TYPE, INT_TYPE);
        dict.insert(ScriptKey::int(1), VmSlot::Int(10));
        dict.insert(ScriptKey::int(2), VmSlot::Int(20));
        dict.insert(ScriptKey::int(3), VmSlot::Int(30));

        // Keep only values > 15
        dict.retain(|_, v| {
            if let VmSlot::Int(n) = v {
                *n > 15
            } else {
                false
            }
        });

        assert_eq!(dict.len(), 2);
        assert!(!dict.contains_key(&ScriptKey::int(1)));
    }

    #[test]
    fn test_clone_dict() {
        let mut dict = ScriptDict::new(STRING_TYPE, INT_TYPE);
        dict.insert(ScriptKey::string("one"), VmSlot::Int(1));
        dict.insert(ScriptKey::string("two"), VmSlot::Int(2));

        let cloned = dict.clone_dict();
        assert_eq!(cloned.len(), 2);
        assert_eq!(cloned.ref_count(), 1); // Fresh ref count
        assert!(matches!(cloned.get(&ScriptKey::string("one")), Some(VmSlot::Int(1))));
    }

    // =========================================================================
    // PREDICATE TESTS
    // =========================================================================

    #[test]
    fn test_contains_value() {
        let mut dict = ScriptDict::new(INT_TYPE, INT_TYPE);
        dict.insert(ScriptKey::int(1), VmSlot::Int(10));
        dict.insert(ScriptKey::int(2), VmSlot::Int(20));

        assert!(dict.contains_value(&VmSlot::Int(10)));
        assert!(!dict.contains_value(&VmSlot::Int(99)));
    }

    #[test]
    fn test_count_value() {
        let mut dict = ScriptDict::new(INT_TYPE, INT_TYPE);
        dict.insert(ScriptKey::int(1), VmSlot::Int(10));
        dict.insert(ScriptKey::int(2), VmSlot::Int(10));
        dict.insert(ScriptKey::int(3), VmSlot::Int(20));

        assert_eq!(dict.count_value(&VmSlot::Int(10)), 2);
        assert_eq!(dict.count_value(&VmSlot::Int(20)), 1);
        assert_eq!(dict.count_value(&VmSlot::Int(99)), 0);
    }

    // =========================================================================
    // FLOAT KEY TESTS
    // =========================================================================

    #[test]
    fn test_float_keys() {
        let mut dict = ScriptDict::new(DOUBLE_TYPE, STRING_TYPE);
        dict.insert(ScriptKey::float(3.14), VmSlot::String("pi".into()));
        dict.insert(ScriptKey::float(2.71), VmSlot::String("e".into()));

        assert!(matches!(
            dict.get(&ScriptKey::float(3.14)),
            Some(VmSlot::String(s)) if s == "pi"
        ));
    }

    #[test]
    fn test_float_key_precision() {
        let mut dict = ScriptDict::new(DOUBLE_TYPE, INT_TYPE);
        dict.insert(ScriptKey::float(0.1 + 0.2), VmSlot::Int(1));

        // Same floating point computation should find the value
        assert!(dict.contains_key(&ScriptKey::float(0.1 + 0.2)));
    }

    // =========================================================================
    // MIXED TYPE TESTS
    // =========================================================================

    #[test]
    fn test_string_to_int_dict() {
        let mut dict = ScriptDict::new(STRING_TYPE, INT_TYPE);
        dict.insert(ScriptKey::string("one"), VmSlot::Int(1));
        dict.insert(ScriptKey::string("two"), VmSlot::Int(2));
        dict.insert(ScriptKey::string("three"), VmSlot::Int(3));

        assert_eq!(dict.len(), 3);
        assert!(matches!(dict.get(&ScriptKey::string("two")), Some(VmSlot::Int(2))));
    }

    #[test]
    fn test_int_to_string_dict() {
        let mut dict = ScriptDict::new(INT_TYPE, STRING_TYPE);
        dict.insert(ScriptKey::int(1), VmSlot::String("one".into()));
        dict.insert(ScriptKey::int(2), VmSlot::String("two".into()));

        assert!(matches!(
            dict.get(&ScriptKey::int(1)),
            Some(VmSlot::String(s)) if s == "one"
        ));
    }

    // =========================================================================
    // DEBUG TESTS
    // =========================================================================

    #[test]
    fn test_debug() {
        let dict = ScriptDict::new(STRING_TYPE, INT_TYPE);
        let debug = format!("{:?}", dict);
        assert!(debug.contains("ScriptDict"));
        assert!(debug.contains("len: 0"));
    }

    // =========================================================================
    // HASHABLE TYPE TESTS
    // =========================================================================

    #[test]
    fn test_is_hashable_type() {
        // Void is not hashable
        assert!(!is_hashable_type(TypeId(0)));

        // Primitives are hashable
        assert!(is_hashable_type(TypeId(1)));  // bool
        assert!(is_hashable_type(TypeId(4)));  // int32
        assert!(is_hashable_type(TypeId(10))); // float
        assert!(is_hashable_type(TypeId(11))); // double

        // String is hashable
        assert!(is_hashable_type(TypeId(16)));
    }

    // =========================================================================
    // EDGE CASE TESTS
    // =========================================================================

    #[test]
    fn test_operations_on_empty_dict() {
        let mut dict = ScriptDict::new(INT_TYPE, INT_TYPE);

        assert!(dict.get(&ScriptKey::int(1)).is_none());
        assert!(dict.remove(&ScriptKey::int(1)).is_none());
        assert!(!dict.contains_key(&ScriptKey::int(1)));
        assert!(!dict.contains_value(&VmSlot::Int(1)));
        assert_eq!(dict.count_value(&VmSlot::Int(1)), 0);
        assert!(dict.keys().is_empty());
        assert!(dict.values().is_empty());
    }

    #[test]
    fn test_null_handle_key() {
        let mut dict = ScriptDict::new(INT_TYPE, INT_TYPE);
        dict.insert(ScriptKey::NullHandle, VmSlot::Int(42));

        assert!(matches!(dict.get(&ScriptKey::NullHandle), Some(VmSlot::Int(42))));
    }

    #[test]
    fn test_bool_keys() {
        let mut dict = ScriptDict::new(BOOL_TYPE, STRING_TYPE);
        dict.insert(ScriptKey::bool(true), VmSlot::String("yes".into()));
        dict.insert(ScriptKey::bool(false), VmSlot::String("no".into()));

        assert_eq!(dict.len(), 2);
        assert!(matches!(
            dict.get(&ScriptKey::bool(true)),
            Some(VmSlot::String(s)) if s == "yes"
        ));
    }

    // =========================================================================
    // FFI MODULE TESTS
    // =========================================================================

    #[test]
    fn test_dictionary_module_creates_successfully() {
        let result = dictionary_module();
        assert!(result.is_ok(), "dictionary module should be created successfully");
    }

    #[test]
    fn test_dictionary_module_is_root_namespace() {
        let module = dictionary_module().expect("dictionary module should build");
        assert!(module.is_root(), "dictionary module should be in root namespace");
    }

    #[test]
    fn test_dictionary_module_has_template() {
        let module = dictionary_module().expect("dictionary module should build");
        let types = module.types();
        assert_eq!(types.len(), 1, "should have exactly one type registered");
        assert_eq!(types[0].name, "dictionary", "type should be named 'dictionary'");
    }

    #[test]
    fn test_dictionary_module_has_methods() {
        let module = dictionary_module().expect("dictionary module should build");
        let ty = &module.types()[0];
        // Should have: getSize, isEmpty, capacity, reserve, shrinkToFit, clear
        assert!(
            ty.methods.len() >= 5,
            "dictionary should have at least 5 methods, got {}",
            ty.methods.len()
        );
    }

    #[test]
    fn test_dictionary_module_method_names() {
        let module = dictionary_module().expect("dictionary module should build");
        let ty = &module.types()[0];
        let method_names: Vec<_> = ty.methods.iter().map(|m| m.name.as_str()).collect();

        assert!(method_names.contains(&"getSize"), "should have getSize method");
        assert!(method_names.contains(&"isEmpty"), "should have isEmpty method");
        assert!(method_names.contains(&"clear"), "should have clear method");
    }

    #[test]
    fn test_dictionary_module_has_behaviors() {
        let module = dictionary_module().expect("dictionary module should build");
        let ty = &module.types()[0];

        assert!(ty.addref.is_some(), "should have addref behavior");
        assert!(ty.release.is_some(), "should have release behavior");
        assert!(ty.list_factory.is_some(), "should have list_factory behavior");
    }

    #[test]
    fn test_native_type_name() {
        assert_eq!(ScriptDict::NAME, "dictionary");
    }

    #[test]
    fn test_is_hashable_type_primitives() {
        // bool is hashable
        assert!(is_hashable_type(BOOL_TYPE));
        // int types are hashable
        assert!(is_hashable_type(INT_TYPE));
        // float types are hashable
        assert!(is_hashable_type(FLOAT_TYPE));
        assert!(is_hashable_type(DOUBLE_TYPE));
        // string is hashable
        assert!(is_hashable_type(STRING_TYPE));
    }

    #[test]
    fn test_is_hashable_type_void_not_hashable() {
        const VOID_TYPE: TypeId = TypeId(0);
        assert!(!is_hashable_type(VOID_TYPE));
    }
}
