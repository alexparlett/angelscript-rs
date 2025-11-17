use crate::core::script_type::ScriptType;
use crate::core::type_registry::TypeRegistry;
use crate::core::types::ScriptValue;
use crate::vm::memory::ObjectHeap;
use std::sync::{Arc, RwLock};

/// asIScriptObject - Represents a script object instance
pub struct ScriptObject {
    handle: u64,
    heap: Arc<RwLock<ObjectHeap>>,
    registry: Arc<RwLock<TypeRegistry>>,
}

impl ScriptObject {
    pub(crate) fn new(
        handle: u64,
        heap: Arc<RwLock<ObjectHeap>>,
        registry: Arc<RwLock<TypeRegistry>>,
    ) -> Self {
        Self {
            handle,
            heap,
            registry,
        }
    }

    pub(crate) fn get_handle(&self) -> u64 {
        self.handle
    }

    pub fn add_ref(&self) -> i32 {
        let heap = self.heap.read().unwrap();
        if let Some(obj) = heap.get_object(self.handle) {
            obj.add_ref();
            1
        } else {
            0
        }
    }

    pub fn release(&self) -> i32 {
        let mut heap = self.heap.write().unwrap();
        if heap.release_object(self.handle) {
            0
        } else {
            1
        }
    }

    pub fn get_type_id(&self) -> i32 {
        let heap = self.heap.read().unwrap();
        heap.get_object(self.handle)
            .map(|obj| obj.type_id() as i32)
            .unwrap_or(0)
    }

    pub fn get_object_type(&self) -> Option<ScriptType> {
        let heap = self.heap.read().unwrap();
        let obj = heap.get_object(self.handle)?;

        Some(ScriptType::new(obj.type_id(), Arc::clone(&self.registry)))
    }

    pub fn get_property_count(&self) -> u32 {
        let heap = self.heap.read().unwrap();
        let obj = match heap.get_object(self.handle) {
            Some(o) => o,
            None => return 0,
        };

        let registry = self.registry.read().unwrap();
        let type_info = match registry.get_type(obj.type_id()) {
            Some(t) => t,
            None => return 0,
        };

        type_info.properties.len() as u32
    }

    pub fn get_property_name(&self, prop: u32) -> Option<String> {
        let heap = self.heap.read().unwrap();
        let obj = heap.get_object(self.handle)?;

        let registry = self.registry.read().unwrap();
        let type_info = registry.get_type(obj.type_id())?;

        type_info
            .properties
            .get(prop as usize)
            .map(|p| p.name.clone())
    }

    pub fn get_property_type_id(&self, prop: u32) -> i32 {
        let heap = self.heap.read().unwrap();
        let obj = match heap.get_object(self.handle) {
            Some(o) => o,
            None => return 0,
        };

        let registry = self.registry.read().unwrap();
        let type_info = match registry.get_type(obj.type_id()) {
            Some(t) => t,
            None => return 0,
        };

        type_info
            .properties
            .get(prop as usize)
            .map(|p| p.type_id as i32)
            .unwrap_or(0)
    }

    pub fn get_address_of_property(&mut self, _prop: u32) -> Option<&mut ScriptValue> {
        None
    }

    pub fn get_property_by_name(&self, name: &str) -> Option<ScriptValue> {
        let heap = self.heap.read().unwrap();
        heap.get_object(self.handle)
            .and_then(|obj| obj.get_property(name))
            .cloned()
    }

    pub fn get_property_by_index(&self, index: u32) -> Option<ScriptValue> {
        let heap = self.heap.read().unwrap();
        let obj = heap.get_object(self.handle)?;

        let registry = self.registry.read().unwrap();
        let type_info = registry.get_type(obj.type_id())?;

        let prop_name = &type_info.properties.get(index as usize)?.name;
        obj.get_property(prop_name).cloned()
    }

    pub fn set_property_by_name(&mut self, name: &str, value: ScriptValue) -> Result<(), String> {
        let mut heap = self.heap.write().unwrap();
        let obj = heap
            .get_object_mut(self.handle)
            .ok_or("Invalid object handle")?;

        obj.set_property(name, value);
        Ok(())
    }

    pub fn set_property_by_index(&mut self, index: u32, value: ScriptValue) -> Result<(), String> {
        let registry = self.registry.read().unwrap();

        let type_id = {
            let heap = self.heap.read().unwrap();
            let obj = heap
                .get_object(self.handle)
                .ok_or("Invalid object handle")?;
            obj.type_id()
        };

        let type_info = registry.get_type(type_id).ok_or("Type not found")?;
        let prop_name = type_info
            .properties
            .get(index as usize)
            .ok_or("Property index out of bounds")?
            .name
            .clone();

        drop(registry);

        let mut heap = self.heap.write().unwrap();
        let obj = heap
            .get_object_mut(self.handle)
            .ok_or("Invalid object handle")?;

        obj.set_property(&prop_name, value);
        Ok(())
    }

    pub fn get_ref_count(&self) -> i32 {
        let heap = self.heap.read().unwrap();
        heap.get_object(self.handle)
            .map(|obj| obj.ref_count() as i32)
            .unwrap_or(0)
    }

    pub fn set_gc_flag(&mut self) {}

    pub fn get_gc_flag(&self) -> bool {
        false
    }

    pub fn enum_references(&self) {}

    pub fn release_all_handles(&mut self) {}
}
