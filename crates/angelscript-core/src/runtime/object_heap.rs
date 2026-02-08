//! Generational arena for reference-counted objects.

use std::any::{Any, TypeId};
use std::fmt;

/// Handle to a heap-allocated object.
///
/// This is a safe, copyable reference to an object in the `ObjectHeap`.
/// The generational index prevents use-after-free bugs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ObjectHandle {
    /// Index into ObjectHeap.slots
    pub index: u32,
    /// Generation for use-after-free detection
    pub generation: u32,
    /// Rust TypeId for runtime type verification and downcasting
    pub type_id: TypeId,
}

impl ObjectHandle {
    /// Create a new object handle.
    pub fn new(index: u32, generation: u32, type_id: TypeId) -> Self {
        Self {
            index,
            generation,
            type_id,
        }
    }
}

/// Heap storage for reference types with generational indices.
///
/// Objects are stored in a Vec with generation tracking. When an object
/// is freed, its slot is reused but the generation is incremented. This
/// allows detecting stale handles at runtime.
pub struct ObjectHeap {
    slots: Vec<HeapSlot>,
    free_list: Vec<u32>,
}

struct HeapSlot {
    generation: u32,
    value: Option<Box<dyn Any + Send + Sync>>,
    ref_count: u32,
}

impl ObjectHeap {
    /// Create a new empty object heap.
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free_list: Vec::new(),
        }
    }

    /// Allocate a new object on the heap.
    pub fn allocate<T: Any + Send + Sync>(&mut self, value: T) -> ObjectHandle {
        let type_id = TypeId::of::<T>();
        let boxed: Box<dyn Any + Send + Sync> = Box::new(value);

        if let Some(index) = self.free_list.pop() {
            let slot = &mut self.slots[index as usize];
            let generation = slot.generation;
            slot.value = Some(boxed);
            slot.ref_count = 1;
            ObjectHandle::new(index, generation, type_id)
        } else {
            let index = self.slots.len() as u32;
            self.slots.push(HeapSlot {
                generation: 0,
                value: Some(boxed),
                ref_count: 1,
            });
            ObjectHandle::new(index, 0, type_id)
        }
    }

    /// Get immutable reference to an object.
    ///
    /// Returns None if the handle is stale or the type doesn't match.
    pub fn get<T: Any>(&self, handle: ObjectHandle) -> Option<&T> {
        let slot = self.slots.get(handle.index as usize)?;
        if slot.generation != handle.generation {
            return None;
        }
        slot.value.as_ref()?.downcast_ref::<T>()
    }

    /// Get mutable reference to an object.
    ///
    /// Returns None if the handle is stale or the type doesn't match.
    pub fn get_mut<T: Any>(&mut self, handle: ObjectHandle) -> Option<&mut T> {
        let slot = self.slots.get_mut(handle.index as usize)?;
        if slot.generation != handle.generation {
            return None;
        }
        slot.value.as_mut()?.downcast_mut::<T>()
    }

    /// Increment reference count.
    pub fn add_ref(&mut self, handle: ObjectHandle) -> bool {
        if let Some(slot) = self.slots.get_mut(handle.index as usize)
            && slot.generation == handle.generation
            && slot.value.is_some()
        {
            slot.ref_count = slot.ref_count.saturating_add(1);
            return true;
        }
        false
    }

    /// Decrement reference count, free if zero.
    ///
    /// Returns true if the object was freed.
    pub fn release(&mut self, handle: ObjectHandle) -> bool {
        if let Some(slot) = self.slots.get_mut(handle.index as usize)
            && slot.generation == handle.generation
            && slot.value.is_some()
        {
            slot.ref_count = slot.ref_count.saturating_sub(1);
            if slot.ref_count == 0 {
                slot.value = None;
                slot.generation = slot.generation.wrapping_add(1);
                self.free_list.push(handle.index);
                return true;
            }
        }
        false
    }

    /// Free object immediately (for scoped types).
    pub fn free(&mut self, handle: ObjectHandle) {
        if let Some(slot) = self.slots.get_mut(handle.index as usize)
            && slot.generation == handle.generation
        {
            slot.value = None;
            slot.generation = slot.generation.wrapping_add(1);
            self.free_list.push(handle.index);
        }
    }

    /// Get the reference count for an object.
    pub fn ref_count(&self, handle: ObjectHandle) -> Option<u32> {
        let slot = self.slots.get(handle.index as usize)?;
        if slot.generation == handle.generation && slot.value.is_some() {
            Some(slot.ref_count)
        } else {
            None
        }
    }
}

impl Default for ObjectHeap {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for ObjectHeap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ObjectHeap")
            .field("slot_count", &self.slots.len())
            .field("free_count", &self.free_list.len())
            .finish()
    }
}
