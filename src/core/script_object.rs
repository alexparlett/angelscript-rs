use std::sync::{Arc, RwLock};
use crate::core::types::ScriptValue;
use crate::vm::memory::ObjectHeap;

pub struct ScriptObject {
    handle: u64,
    heap: Arc<RwLock<ObjectHeap>>,
}

impl ScriptObject {
    pub fn get_type_id(&self) -> u32 {
        let heap = self.heap.read().unwrap();
        heap.get_object(self.handle)
            .map(|obj| obj.type_id())
            .unwrap_or(0)
    }

    /// Get number of properties
    pub fn get_property_count(&self) -> u32 {
        let heap = self.heap.read().unwrap();
        heap.get_object(self.handle)
            .map(|obj| obj.properties().len() as u32)
            .unwrap_or(0)
    }

    /// Get property name by index
    pub fn get_property_name(&self, prop: u32) -> Option<String> {
        let heap = self.heap.read().unwrap();
        heap.get_object(self.handle)
            .and_then(|obj| {
                obj.properties()
                   .keys()
                   .nth(prop as usize)
                   .cloned()
            })
    }

    /// Get property type ID by index
    pub fn get_property_type_id(&self, prop: u32) -> u32 {
        // Would need to store type info in properties
        0
    }

    /// Get property value by index (returns mutable reference in Rust)
    pub fn get_property_mut(&mut self, prop: u32) -> Option<&mut ScriptValue> {
        let mut heap = self.heap.write().unwrap();
        heap.get_object_mut(self.handle)
            .and_then(|obj| {
                obj.properties_mut()
                   .values_mut()
                   .nth(prop as usize)
            })
    }

    /// Get property value by index (immutable)
    pub fn get_property(&self, prop: u32) -> Option<ScriptValue> {
        let heap = self.heap.read().unwrap();
        heap.get_object(self.handle)
            .and_then(|obj| {
                obj.properties()
                   .values()
                   .nth(prop as usize)
                   .cloned()
            })
    }

    /// Helper: Get property by name (not in C++ API, but useful)
    pub fn get_property_by_name(&self, name: &str) -> Option<ScriptValue> {
        let heap = self.heap.read().unwrap();
        heap.get_object(self.handle)
            .and_then(|obj| obj.get_property(name))
            .cloned()
    }

    pub fn add_ref(&self) {
        let heap = self.heap.read().unwrap();
        if let Some(obj) = heap.get_object(self.handle) {
            obj.add_ref();
        }
    }

    pub fn release(&self) -> bool {
        let mut heap = self.heap.write().unwrap();
        heap.release_object(self.handle)
    }
}
