//! Memory management for the AngelScript VM
//!
//! This module provides:
//! - HeapEntry: Type-erased storage for heap objects
//! - ObjectHeap: Manages all heap allocations as `Box<dyn Any + Send + Sync>`
//!
//! ## Architecture
//!
//! The heap is completely type-agnostic. It stores all objects uniformly as
//! `Box<dyn Any + Send + Sync>` and provides only:
//! - Storage (allocate, remove, contains)
//! - Type metadata (get_type_id, is_gc_tracked)
//! - Raw access (get_as_any, get_as_any_mut)
//!
//! ALL operations on objects (AddRef, Release, GetProperty, etc.) must go
//! through `call_system_function()` using the registered behaviours. The heap
//! does not know or care what types are stored - it treats ScriptObject and
//! application types identically.
//!
//! ## Usage Pattern
//!
//! ```ignore
//! // 1. Get type info to find behaviour FunctionId
//! let type_id = heap.get_type_id(handle)?;
//! let type_info = registry.get_type(type_id)?;
//! let add_ref_id = type_info.get_behaviour(BehaviourType::AddRef)?;
//!
//! // 2. Get object as raw &mut dyn Any
//! let obj = heap.get_as_any_mut(handle)?;
//!
//! // 3. Call behaviour via call_system_function
//! call_system_function(add_ref_id, Some(obj), &[], &system_functions)?;
//! ```

use crate::core::types::TypeId;
use std::any::Any;
use std::collections::HashMap;

// ============================================================================
// HeapEntry - Type-erased storage for any object
// ============================================================================

/// A single entry in the object heap
///
/// Objects are stored as type-erased `Box<dyn Any + Send + Sync>`:
/// - Script objects are stored as `ScriptObject`
/// - Application types are stored as their concrete Rust types
///
/// The heap does not interact with the contents - all operations go through
/// `call_system_function()`.
struct HeapEntry {
    /// The actual object data (ScriptObject, Player, etc.)
    data: Box<dyn Any + Send + Sync>,
    /// Type ID for looking up type info and behaviours
    type_id: TypeId,
    /// Whether this object is tracked by the GC
    gc_tracked: bool,
}

// ============================================================================
// ObjectHeap - Type-agnostic heap storage
// ============================================================================

/// The object heap manages all allocated objects
///
/// This is a simple storage container that:
/// - Allocates objects and assigns handles
/// - Stores type metadata (type_id, gc_tracked)
/// - Provides raw access to objects as `&dyn Any` / `&mut dyn Any`
///
/// The heap does NOT:
/// - Call any methods on stored objects
/// - Know anything about ScriptObject vs application types
/// - Perform reference counting or GC operations
///
/// All object operations must go through `call_system_function()` using
/// the appropriate behaviour FunctionIds from the type registry.
pub struct ObjectHeap {
    /// Objects stored as type-erased boxes, keyed by handle
    objects: HashMap<u64, HeapEntry>,
    /// Next handle to allocate
    next_handle: u64,
}

impl ObjectHeap {
    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
            next_handle: 1,
        }
    }

    /// Generate a new unique object handle
    fn generate_handle(&mut self) -> u64 {
        let handle = self.next_handle;
        self.next_handle += 1;
        handle
    }

    // ========================================================================
    // Allocation
    // ========================================================================

    /// Allocate an object on the heap
    ///
    /// The object is stored as `Box<dyn Any + Send + Sync>`.
    /// Returns the handle for accessing the object.
    ///
    /// After allocation, the caller should:
    /// 1. Call the Construct behaviour if needed
    /// 2. Register with GC if gc_tracked is true
    pub fn allocate<T: Any + Send + Sync>(
        &mut self,
        type_id: TypeId,
        value: T,
        gc_tracked: bool,
    ) -> u64 {
        let handle = self.generate_handle();
        let entry = HeapEntry {
            data: Box::new(value),
            type_id,
            gc_tracked,
        };
        self.objects.insert(handle, entry);
        handle
    }

    /// Remove an object from the heap
    ///
    /// This only removes the object from storage. The caller is responsible
    /// for calling Release/Destruct behaviours before removal.
    ///
    /// Returns true if the object existed and was removed.
    pub fn remove(&mut self, handle: u64) -> bool {
        self.objects.remove(&handle).is_some()
    }

    // ========================================================================
    // Metadata access
    // ========================================================================

    /// Check if an object exists on the heap
    pub fn contains(&self, handle: u64) -> bool {
        self.objects.contains_key(&handle)
    }

    /// Get the type ID for an object
    pub fn get_type_id(&self, handle: u64) -> Option<TypeId> {
        self.objects.get(&handle).map(|e| e.type_id)
    }

    /// Check if an object is GC tracked
    pub fn is_gc_tracked(&self, handle: u64) -> Option<bool> {
        self.objects.get(&handle).map(|e| e.gc_tracked)
    }

    /// Get all handles on the heap
    pub fn all_handles(&self) -> Vec<u64> {
        self.objects.keys().copied().collect()
    }

    /// Get all GC-tracked handles
    pub fn gc_tracked_handles(&self) -> Vec<u64> {
        self.objects
            .iter()
            .filter(|(_, e)| e.gc_tracked)
            .map(|(h, _)| *h)
            .collect()
    }

    /// Get handle and type_id pairs for all objects
    pub fn all_handles_with_types(&self) -> Vec<(u64, TypeId)> {
        self.objects
            .iter()
            .map(|(h, e)| (*h, e.type_id))
            .collect()
    }

    /// Get handle and type_id pairs for GC-tracked objects
    pub fn gc_tracked_handles_with_types(&self) -> Vec<(u64, TypeId)> {
        self.objects
            .iter()
            .filter(|(_, e)| e.gc_tracked)
            .map(|(h, e)| (*h, e.type_id))
            .collect()
    }

    /// Get the number of objects on the heap
    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    // ========================================================================
    // Raw object access (for call_system_function)
    // ========================================================================

    /// Get a reference to an object's data as `&dyn Any`
    ///
    /// Use this with `call_system_function()` to call behaviours.
    pub fn get_as_any(&self, handle: u64) -> Option<&dyn Any> {
        self.objects
            .get(&handle)
            .map(|e| e.data.as_ref() as &dyn Any)
    }

    /// Get a mutable reference to an object's data as `&mut dyn Any`
    ///
    /// Use this with `call_system_function()` to call behaviours.
    pub fn get_as_any_mut(&mut self, handle: u64) -> Option<&mut dyn Any> {
        self.objects
            .get_mut(&handle)
            .map(|e| e.data.as_mut() as &mut dyn Any)
    }

    /// Get a typed reference to an object's data
    ///
    /// This is a convenience method that downcasts the object.
    /// Prefer using `get_as_any()` + `call_system_function()` for behaviours.
    pub fn get_as<T: Any>(&self, handle: u64) -> Option<&T> {
        self.objects
            .get(&handle)
            .and_then(|e| e.data.downcast_ref::<T>())
    }

    /// Get a typed mutable reference to an object's data
    ///
    /// This is a convenience method that downcasts the object.
    /// Prefer using `get_as_any_mut()` + `call_system_function()` for behaviours.
    pub fn get_as_mut<T: Any>(&mut self, handle: u64) -> Option<&mut T> {
        self.objects
            .get_mut(&handle)
            .and_then(|e| e.data.downcast_mut::<T>())
    }
}

impl Default for ObjectHeap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct TestObject {
        value: i32,
    }

    #[test]
    fn test_allocate_and_contains() {
        let mut heap = ObjectHeap::new();
        let handle = heap.allocate(100, TestObject { value: 42 }, false);

        assert!(heap.contains(handle));
        assert!(!heap.contains(handle + 1));
    }

    #[test]
    fn test_get_type_id() {
        let mut heap = ObjectHeap::new();
        let handle = heap.allocate(100, TestObject { value: 42 }, false);

        assert_eq!(heap.get_type_id(handle), Some(100));
        assert_eq!(heap.get_type_id(handle + 1), None);
    }

    #[test]
    fn test_is_gc_tracked() {
        let mut heap = ObjectHeap::new();
        let handle1 = heap.allocate(100, TestObject { value: 1 }, false);
        let handle2 = heap.allocate(100, TestObject { value: 2 }, true);

        assert_eq!(heap.is_gc_tracked(handle1), Some(false));
        assert_eq!(heap.is_gc_tracked(handle2), Some(true));
    }

    #[test]
    fn test_remove() {
        let mut heap = ObjectHeap::new();
        let handle = heap.allocate(100, TestObject { value: 42 }, false);

        assert!(heap.contains(handle));
        assert!(heap.remove(handle));
        assert!(!heap.contains(handle));
        assert!(!heap.remove(handle)); // Already removed
    }

    #[test]
    fn test_get_as_any() {
        let mut heap = ObjectHeap::new();
        let handle = heap.allocate(100, TestObject { value: 42 }, false);

        let any_ref = heap.get_as_any(handle).unwrap();
        let obj = any_ref.downcast_ref::<TestObject>().unwrap();
        assert_eq!(obj.value, 42);
    }

    #[test]
    fn test_get_as_any_mut() {
        let mut heap = ObjectHeap::new();
        let handle = heap.allocate(100, TestObject { value: 42 }, false);

        {
            let any_mut = heap.get_as_any_mut(handle).unwrap();
            let obj = any_mut.downcast_mut::<TestObject>().unwrap();
            obj.value = 100;
        }

        let obj = heap.get_as::<TestObject>(handle).unwrap();
        assert_eq!(obj.value, 100);
    }

    #[test]
    fn test_get_as_typed() {
        let mut heap = ObjectHeap::new();
        let handle = heap.allocate(100, TestObject { value: 42 }, false);

        let obj = heap.get_as::<TestObject>(handle).unwrap();
        assert_eq!(obj.value, 42);

        // Wrong type returns None
        assert!(heap.get_as::<String>(handle).is_none());
    }

    #[test]
    fn test_all_handles() {
        let mut heap = ObjectHeap::new();
        let h1 = heap.allocate(100, TestObject { value: 1 }, false);
        let h2 = heap.allocate(100, TestObject { value: 2 }, true);
        let h3 = heap.allocate(200, TestObject { value: 3 }, true);

        let all = heap.all_handles();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&h1));
        assert!(all.contains(&h2));
        assert!(all.contains(&h3));

        let gc_tracked = heap.gc_tracked_handles();
        assert_eq!(gc_tracked.len(), 2);
        assert!(!gc_tracked.contains(&h1));
        assert!(gc_tracked.contains(&h2));
        assert!(gc_tracked.contains(&h3));
    }

    #[test]
    fn test_object_count() {
        let mut heap = ObjectHeap::new();
        assert_eq!(heap.object_count(), 0);

        let h1 = heap.allocate(100, TestObject { value: 1 }, false);
        assert_eq!(heap.object_count(), 1);

        let _h2 = heap.allocate(100, TestObject { value: 2 }, false);
        assert_eq!(heap.object_count(), 2);

        heap.remove(h1);
        assert_eq!(heap.object_count(), 1);
    }

    #[test]
    fn test_handles_are_unique() {
        let mut heap = ObjectHeap::new();
        let h1 = heap.allocate(100, TestObject { value: 1 }, false);
        let h2 = heap.allocate(100, TestObject { value: 2 }, false);
        let h3 = heap.allocate(100, TestObject { value: 3 }, false);

        assert_ne!(h1, h2);
        assert_ne!(h2, h3);
        assert_ne!(h1, h3);
    }
}