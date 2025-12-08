//! ScriptDict - FFI registration for AngelScript dictionary<K,V> template.
//!
//! This is a placeholder implementation for FFI registration.
//! The actual storage and runtime implementation will be handled by the VM.

use angelscript_core::AnyRef;
use angelscript_macros::Any;
use angelscript_registry::Module;

/// Placeholder for AngelScript `dictionary<K,V>` template.
///
/// This is an empty struct used purely for FFI registration.
/// The actual implementation will be provided by the VM.
#[derive(Any)]
#[angelscript(name = "dictionary", reference, template = "<K, V>")]
pub struct ScriptDict;

impl ScriptDict {
    // =========================================================================
    // REFERENCE COUNTING
    // =========================================================================

    /// Increment reference count.
    #[angelscript_macros::function(addref)]
    pub fn add_ref(&self) {
        todo!()
    }

    /// Decrement reference count.
    #[angelscript_macros::function(release)]
    pub fn release(&self) -> bool {
        todo!()
    }

    // =========================================================================
    // SIZE AND CAPACITY
    // =========================================================================

    /// Returns the number of entries.
    #[angelscript_macros::function(instance, const, name = "getSize")]
    pub fn len(&self) -> u32 {
        todo!()
    }

    /// Returns true if the dictionary is empty.
    #[angelscript_macros::function(instance, const, name = "isEmpty")]
    pub fn is_empty(&self) -> bool {
        todo!()
    }

    /// Returns the allocated capacity.
    #[angelscript_macros::function(instance, const)]
    pub fn capacity(&self) -> u32 {
        todo!()
    }

    /// Reserve capacity for at least `count` entries.
    #[angelscript_macros::function(instance)]
    pub fn reserve(&mut self, count: u32) {
        let _ = count;
        todo!()
    }

    /// Shrink capacity to fit current number of entries.
    #[angelscript_macros::function(instance, name = "shrinkToFit")]
    pub fn shrink_to_fit(&mut self) {
        todo!()
    }

    /// Remove all entries.
    #[angelscript_macros::function(instance)]
    pub fn clear(&mut self) {
        todo!()
    }

    // =========================================================================
    // TEMPLATE PARAMETER METHODS
    // =========================================================================

    /// Insert or update an entry.
    #[angelscript_macros::function(instance, name = "set")]
    pub fn set(&mut self, #[template("K")] key: AnyRef<'static>, #[template("V")] value: AnyRef<'static>) {
        let _ = (key, value);
        todo!()
    }

    /// Check if key exists.
    #[angelscript_macros::function(instance, const)]
    pub fn exists(&self, #[template("K")] key: AnyRef<'static>) -> bool {
        let _ = key;
        todo!()
    }

    /// Get value by key (returns bool indicating success, value via out param).
    #[angelscript_macros::function(instance, const)]
    pub fn get(&self, #[template("K")] key: AnyRef<'static>, #[template("V")] out_value: AnyRef<'static>) -> bool {
        let _ = (key, out_value);
        todo!()
    }

    /// Delete entry by key.
    #[angelscript_macros::function(instance)]
    pub fn delete(&mut self, #[template("K")] key: AnyRef<'static>) -> bool {
        let _ = key;
        todo!()
    }

    // =========================================================================
    // OPERATORS
    // =========================================================================

    /// Index operator (mutable).
    #[angelscript_macros::function(instance, operator = angelscript_core::Operator::Index)]
    pub fn op_index(&mut self, #[template("K")] key: AnyRef<'static>) -> AnyRef<'static> {
        let _ = key;
        todo!()
    }

    /// Index operator (const).
    #[angelscript_macros::function(instance, const, operator = angelscript_core::Operator::Index)]
    pub fn op_index_const(&self, #[template("K")] key: AnyRef<'static>) -> AnyRef<'static> {
        let _ = key;
        todo!()
    }

    /// Equality comparison.
    #[angelscript_macros::function(instance, const, operator = angelscript_core::Operator::Equals)]
    pub fn op_equals(&self, other: &Self) -> bool {
        let _ = other;
        todo!()
    }

    /// Assignment operator.
    #[angelscript_macros::function(instance, operator = angelscript_core::Operator::Assign)]
    pub fn op_assign(&mut self, other: &Self) {
        let _ = other;
        todo!()
    }
}

// =========================================================================
// MODULE CREATION
// =========================================================================

/// Creates the dictionary module with the `dictionary<K,V>` template type.
pub fn module() -> Module {
    Module::new()
        .ty::<ScriptDict>()
        // Reference counting
        .function(ScriptDict::add_ref__meta)
        .function(ScriptDict::release__meta)
        // Size/capacity operations
        .function(ScriptDict::len__meta)
        .function(ScriptDict::is_empty__meta)
        .function(ScriptDict::capacity__meta)
        .function(ScriptDict::reserve__meta)
        .function(ScriptDict::shrink_to_fit__meta)
        .function(ScriptDict::clear__meta)
        // Template parameter methods
        .function(ScriptDict::set__meta)
        .function(ScriptDict::exists__meta)
        .function(ScriptDict::get__meta)
        .function(ScriptDict::delete__meta)
        // Operators
        .function(ScriptDict::op_index__meta)
        .function(ScriptDict::op_index_const__meta)
        .function(ScriptDict::op_equals__meta)
        .function(ScriptDict::op_assign__meta)
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_registry::HasClassMeta;

    #[test]
    fn test_module_creates() {
        let meta = ScriptDict::__as_type_meta();
        assert_eq!(meta.name, "dictionary");
    }
}
