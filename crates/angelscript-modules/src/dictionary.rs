//! ScriptDict - Type-erased, reference-counted dictionary for AngelScript.
//!
//! This is a REFERENCE type - passed by handle with manual reference counting.
//! Keys must be hashable types (primitives, strings, handles).

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};

use ordered_float::OrderedFloat;

use angelscript_core::TypeHash;
use angelscript_macros::Any;
use angelscript_registry::Module;

/// Hashable wrapper for dictionary keys.
///
/// Only primitives, strings, and handles are valid keys.
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
    /// Null handle key
    NullHandle,
}

impl ScriptKey {
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

    /// Create a null handle key.
    #[inline]
    pub fn null() -> Self {
        ScriptKey::NullHandle
    }
}

/// Type-erased dictionary for AngelScript `dictionary<K,V>` template.
///
/// This is a REFERENCE type with manual reference counting.
/// Keys are stored as `ScriptKey` (hashable) and values as raw bytes.
#[derive(Any)]
#[angelscript(name = "dictionary", reference, template = "<K, V>")]
pub struct ScriptDict {
    /// Type-erased storage
    entries: HashMap<ScriptKey, Vec<u8>>,
    /// Value size in bytes
    value_size: usize,
    /// Key type for runtime checking
    key_type_id: TypeHash,
    /// Value type for runtime checking
    value_type_id: TypeHash,
    /// Reference count (starts at 1)
    ref_count: AtomicU32,
}

impl ScriptDict {
    // =========================================================================
    // CONSTRUCTORS
    // =========================================================================

    /// Create an empty dictionary for given key and value types.
    pub fn new(key_type_id: TypeHash, value_type_id: TypeHash, value_size: usize) -> Self {
        Self {
            entries: HashMap::new(),
            value_size,
            key_type_id,
            value_type_id,
            ref_count: AtomicU32::new(1),
        }
    }

    /// Create dictionary with initial capacity.
    pub fn with_capacity(
        key_type_id: TypeHash,
        value_type_id: TypeHash,
        value_size: usize,
        capacity: usize,
    ) -> Self {
        Self {
            entries: HashMap::with_capacity(capacity),
            value_size,
            key_type_id,
            value_type_id,
            ref_count: AtomicU32::new(1),
        }
    }

    /// Get the key type ID.
    #[inline]
    pub fn key_type_id(&self) -> TypeHash {
        self.key_type_id
    }

    /// Get the value type ID.
    #[inline]
    pub fn value_type_id(&self) -> TypeHash {
        self.value_type_id
    }

    /// Get the value size in bytes.
    #[inline]
    pub fn value_size(&self) -> usize {
        self.value_size
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

    /// Returns the number of entries.
    #[angelscript_macros::function(instance, const, name = "getSize")]
    pub fn len(&self) -> u32 {
        self.entries.len() as u32
    }

    /// Returns true if the dictionary is empty.
    #[angelscript_macros::function(instance, const, name = "isEmpty")]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the allocated capacity.
    #[angelscript_macros::function(instance, const)]
    pub fn capacity(&self) -> u32 {
        self.entries.capacity() as u32
    }

    /// Reserve capacity for at least `additional` more entries.
    #[angelscript_macros::function(instance)]
    pub fn reserve(&mut self, additional: u32) {
        self.entries.reserve(additional as usize);
    }

    /// Shrink capacity to fit current number of entries.
    #[angelscript_macros::function(instance, name = "shrinkToFit")]
    pub fn shrink_to_fit(&mut self) {
        self.entries.shrink_to_fit();
    }

    // =========================================================================
    // INSERTION AND RETRIEVAL (raw bytes interface)
    // =========================================================================

    /// Insert or update an entry with raw value bytes.
    ///
    /// # Safety
    /// The bytes must represent a valid value of the value type.
    pub unsafe fn insert_raw(&mut self, key: ScriptKey, value: &[u8]) {
        debug_assert_eq!(value.len(), self.value_size);
        self.entries.insert(key, value.to_vec());
    }

    /// Get raw value bytes by key.
    pub fn get_raw(&self, key: &ScriptKey) -> Option<&[u8]> {
        self.entries.get(key).map(|v| v.as_slice())
    }

    /// Get mutable raw value bytes by key.
    pub fn get_raw_mut(&mut self, key: &ScriptKey) -> Option<&mut [u8]> {
        self.entries.get_mut(key).map(|v| v.as_mut_slice())
    }

    /// Check if key exists.
    /// Note: Not exposed to AngelScript directly - VM uses this via generic calling convention
    pub fn exists(&self, key: &ScriptKey) -> bool {
        self.entries.contains_key(key)
    }

    // =========================================================================
    // REMOVAL
    // =========================================================================

    /// Remove entry and return true if it existed.
    pub fn delete(&mut self, key: &ScriptKey) -> bool {
        self.entries.remove(key).is_some()
    }

    /// Remove all entries.
    #[angelscript_macros::function(instance)]
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    // =========================================================================
    // KEY ACCESS
    // =========================================================================

    /// Get all keys.
    pub fn keys(&self) -> impl Iterator<Item = &ScriptKey> {
        self.entries.keys()
    }

    /// Iterate over key-value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&ScriptKey, &[u8])> {
        self.entries.iter().map(|(k, v)| (k, v.as_slice()))
    }
}

impl fmt::Debug for ScriptDict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScriptDict")
            .field("key_type_id", &self.key_type_id)
            .field("value_type_id", &self.value_type_id)
            .field("value_size", &self.value_size)
            .field("len", &self.entries.len())
            .field("ref_count", &self.ref_count.load(AtomicOrdering::Relaxed))
            .finish()
    }
}

// =========================================================================
// MODULE CREATION
// =========================================================================

/// Creates the dictionary module with the `dictionary<K,V>` template type.
///
/// Registers the built-in dictionary template with:
/// - Reference counting behaviors (addref/release)
/// - Basic size/capacity methods
///
/// # Example
///
/// ```ignore
/// use angelscript_modules::dictionary;
///
/// let module = dictionary::module();
/// // Install with context...
/// ```
pub fn module() -> Module {
    Module::new().class::<ScriptDict>()
}

// =========================================================================
// TESTS
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::primitives;

    #[test]
    fn test_new_creates_empty_dict() {
        let dict = ScriptDict::new(primitives::STRING, primitives::INT32, 4);
        assert!(dict.is_empty());
        assert_eq!(dict.len(), 0);
        assert_eq!(dict.key_type_id(), primitives::STRING);
        assert_eq!(dict.value_type_id(), primitives::INT32);
        assert_eq!(dict.value_size(), 4);
        assert_eq!(dict.ref_count(), 1);
    }

    #[test]
    fn test_ref_count() {
        let dict = ScriptDict::new(primitives::INT32, primitives::INT32, 4);
        assert_eq!(dict.ref_count(), 1);
        dict.add_ref();
        assert_eq!(dict.ref_count(), 2);
        assert!(!dict.release());
        assert_eq!(dict.ref_count(), 1);
        assert!(dict.release());
    }

    #[test]
    fn test_insert_and_get() {
        let mut dict = ScriptDict::new(primitives::STRING, primitives::INT32, 4);
        unsafe {
            dict.insert_raw(ScriptKey::string("hello"), &42i32.to_ne_bytes());
        }
        assert_eq!(dict.len(), 1);
        assert!(dict.exists(&ScriptKey::string("hello")));

        let value = dict.get_raw(&ScriptKey::string("hello")).unwrap();
        let v = i32::from_ne_bytes(value.try_into().unwrap());
        assert_eq!(v, 42);
    }

    #[test]
    fn test_delete() {
        let mut dict = ScriptDict::new(primitives::STRING, primitives::INT32, 4);
        unsafe {
            dict.insert_raw(ScriptKey::string("key"), &100i32.to_ne_bytes());
        }
        assert!(dict.delete(&ScriptKey::string("key")));
        assert!(dict.is_empty());
        assert!(!dict.delete(&ScriptKey::string("key")));
    }

    #[test]
    fn test_clear() {
        let mut dict = ScriptDict::new(primitives::INT32, primitives::INT32, 4);
        unsafe {
            dict.insert_raw(ScriptKey::int(1), &10i32.to_ne_bytes());
            dict.insert_raw(ScriptKey::int(2), &20i32.to_ne_bytes());
        }
        dict.clear();
        assert!(dict.is_empty());
    }

    #[test]
    fn test_module_creates() {
        use angelscript_registry::HasClassMeta;
        let meta = ScriptDict::__as_type_meta();
        assert_eq!(meta.name, "dictionary");
    }

    #[test]
    fn test_script_key_variants() {
        let _ = ScriptKey::int(42);
        let _ = ScriptKey::float(3.14);
        let _ = ScriptKey::bool(true);
        let _ = ScriptKey::string("test");
        let _ = ScriptKey::null();
    }
}
