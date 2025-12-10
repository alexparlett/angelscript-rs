//! ScriptArray - FFI registration for AngelScript array<T> template.
//!
//! This is a placeholder implementation for FFI registration.
//! The actual storage and runtime implementation will be handled by the VM.

use angelscript_core::Dynamic;
use angelscript_macros::Any;
use angelscript_registry::Module;

/// Placeholder for AngelScript `array<T>` template.
///
/// This is an empty struct used purely for FFI registration.
/// The actual implementation will be provided by the VM.
#[derive(Any)]
#[angelscript(name = "array", reference, template = "<T>")]
pub struct ScriptArray;

impl ScriptArray {
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

    /// Returns the number of elements.
    #[angelscript_macros::function(instance, const, name = "length")]
    pub fn len(&self) -> u32 {
        todo!()
    }

    /// Returns true if the array is empty.
    #[angelscript_macros::function(instance, const, name = "isEmpty")]
    pub fn is_empty(&self) -> bool {
        todo!()
    }

    /// Returns the allocated capacity.
    #[angelscript_macros::function(instance, const)]
    pub fn capacity(&self) -> u32 {
        todo!()
    }

    /// Reserve capacity for at least `count` elements.
    #[angelscript_macros::function(instance)]
    pub fn reserve(&mut self, count: u32) {
        let _ = count;
        todo!()
    }

    /// Resize array to `count` elements.
    #[angelscript_macros::function(instance)]
    pub fn resize(&mut self, count: u32) {
        let _ = count;
        todo!()
    }

    /// Shrink capacity to fit current length.
    #[angelscript_macros::function(instance, name = "shrinkToFit")]
    pub fn shrink_to_fit(&mut self) {
        todo!()
    }

    /// Remove all elements.
    #[angelscript_macros::function(instance)]
    pub fn clear(&mut self) {
        todo!()
    }

    // =========================================================================
    // REMOVAL
    // =========================================================================

    /// Remove element at position.
    #[angelscript_macros::function(instance, name = "removeAt")]
    pub fn remove_at(&mut self, index: u32) {
        let _ = index;
        todo!()
    }

    /// Remove the last element.
    #[angelscript_macros::function(instance, name = "removeLast")]
    pub fn remove_last(&mut self) {
        todo!()
    }

    /// Remove range of elements [start..start+count].
    #[angelscript_macros::function(instance, name = "removeRange")]
    pub fn remove_range(&mut self, start: u32, count: u32) {
        let _ = (start, count);
        todo!()
    }

    // =========================================================================
    // ORDERING
    // =========================================================================

    /// Reverse elements in place.
    #[angelscript_macros::function(instance)]
    pub fn reverse(&mut self) {
        todo!()
    }

    /// Sort elements in ascending order.
    #[angelscript_macros::function(instance, name = "sortAsc")]
    pub fn sort_asc(&mut self) {
        todo!()
    }

    /// Sort elements in descending order.
    #[angelscript_macros::function(instance, name = "sortDesc")]
    pub fn sort_desc(&mut self) {
        todo!()
    }

    // =========================================================================
    // TEMPLATE PARAMETER METHODS
    // =========================================================================

    /// Insert element at position.
    #[angelscript_macros::function(instance, name = "insertAt")]
    pub fn insert_at(&mut self, index: u32, #[template("T")] value: Dynamic) {
        let _ = (index, value);
        todo!()
    }

    /// Insert element at the end.
    #[angelscript_macros::function(instance, name = "insertLast")]
    pub fn insert_last(&mut self, #[template("T")] value: Dynamic) {
        let _ = value;
        todo!()
    }

    /// Find first occurrence of value.
    #[angelscript_macros::function(instance, const)]
    pub fn find(&self, #[template("T")] value: Dynamic) -> i32 {
        let _ = value;
        todo!()
    }

    /// Find first occurrence of value starting from `start`.
    #[angelscript_macros::function(instance, const, name = "findFrom")]
    pub fn find_from(&self, start: u32, #[template("T")] value: Dynamic) -> i32 {
        let _ = (start, value);
        todo!()
    }

    /// Check if array contains value.
    #[angelscript_macros::function(instance, const)]
    pub fn contains(&self, #[template("T")] value: Dynamic) -> bool {
        let _ = value;
        todo!()
    }

    // =========================================================================
    // OPERATORS
    // =========================================================================

    /// Index operator (mutable).
    #[angelscript_macros::function(instance, operator = Operator::Index)]
    #[returns(template = "T")]
    pub fn op_index(&mut self, index: u32) -> Dynamic {
        let _ = index;
        todo!()
    }

    /// Index operator (const).
    #[angelscript_macros::function(instance, const, operator = Operator::Index)]
    #[returns(template = "T")]
    pub fn op_index_const(&self, index: u32) -> Dynamic {
        let _ = index;
        todo!()
    }

    /// Equality comparison.
    #[angelscript_macros::function(instance, const, operator = Operator::Equals)]
    pub fn op_equals(&self, other: &Self) -> bool {
        let _ = other;
        todo!()
    }

    /// Assignment operator.
    #[angelscript_macros::function(instance, operator = Operator::Assign)]
    pub fn op_assign(&mut self, other: &Self) {
        let _ = other;
        todo!()
    }
}

// =========================================================================
// MODULE CREATION
// =========================================================================

/// Creates the array module with the `array<T>` template type.
pub fn module() -> Module {
    Module::new()
        .ty::<ScriptArray>()
        // Reference counting
        .function(ScriptArray::add_ref__meta)
        .function(ScriptArray::release__meta)
        // Size/capacity operations
        .function(ScriptArray::len__meta)
        .function(ScriptArray::is_empty__meta)
        .function(ScriptArray::capacity__meta)
        .function(ScriptArray::reserve__meta)
        .function(ScriptArray::resize__meta)
        .function(ScriptArray::shrink_to_fit__meta)
        .function(ScriptArray::clear__meta)
        // Removal
        .function(ScriptArray::remove_at__meta)
        .function(ScriptArray::remove_last__meta)
        .function(ScriptArray::remove_range__meta)
        // Ordering
        .function(ScriptArray::reverse__meta)
        .function(ScriptArray::sort_asc__meta)
        .function(ScriptArray::sort_desc__meta)
        // Template parameter methods
        .function(ScriptArray::insert_at__meta)
        .function(ScriptArray::insert_last__meta)
        .function(ScriptArray::find__meta)
        .function(ScriptArray::find_from__meta)
        .function(ScriptArray::contains__meta)
        // Operators
        .function(ScriptArray::op_index__meta)
        .function(ScriptArray::op_index_const__meta)
        .function(ScriptArray::op_equals__meta)
        .function(ScriptArray::op_assign__meta)
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_registry::HasClassMeta;

    #[test]
    fn test_module_creates() {
        let meta = ScriptArray::__as_type_meta();
        assert_eq!(meta.name, "array");
    }
}
