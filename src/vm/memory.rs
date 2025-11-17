use crate::core::type_registry::TypeRegistry;
use crate::core::types::{ScriptValue, TypeId};
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct Object {
    type_id: TypeId,
    properties: HashMap<String, ScriptValue>,
    ref_count: Arc<RwLock<usize>>,
    rust_backing: Option<Arc<RwLock<Box<dyn Any + Send + Sync>>>>,
    object_id: u64,
}

impl Object {
    pub fn new_script(type_id: TypeId) -> Self {
        Self {
            type_id,
            properties: HashMap::new(),
            ref_count: Arc::new(RwLock::new(1)),
            rust_backing: None,
            object_id: Self::generate_object_id(),
        }
    }

    pub fn new_rust<T: Any + Send + Sync>(type_id: TypeId, rust_instance: T) -> Self {
        Self {
            type_id,
            properties: HashMap::new(),
            ref_count: Arc::new(RwLock::new(1)),
            rust_backing: Some(Arc::new(RwLock::new(Box::new(rust_instance)))),
            object_id: Self::generate_object_id(),
        }
    }

    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    pub fn object_id(&self) -> u64 {
        self.object_id
    }

    pub fn is_rust_backed(&self) -> bool {
        self.rust_backing.is_some()
    }

    pub fn get_property(&self, name: &str) -> Option<&ScriptValue> {
        self.properties.get(name)
    }

    pub fn set_property(&mut self, name: &str, value: ScriptValue) {
        self.properties.insert(name.to_string(), value);
    }

    pub fn properties(&self) -> &HashMap<String, ScriptValue> {
        &self.properties
    }

    pub fn properties_mut(&mut self) -> &mut HashMap<String, ScriptValue> {
        &mut self.properties
    }

    pub fn rust_backing(&self) -> Option<Arc<RwLock<Box<dyn Any + Send + Sync>>>> {
        self.rust_backing.clone()
    }

    pub fn add_ref(&self) {
        let mut count = self.ref_count.write().unwrap();
        *count += 1;
    }

    pub fn release(&self) -> bool {
        let mut count = self.ref_count.write().unwrap();
        *count = count.saturating_sub(1);
        *count == 0
    }

    pub fn ref_count(&self) -> usize {
        *self.ref_count.read().unwrap()
    }

    fn generate_object_id() -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    }
}

pub struct ObjectHeap {
    objects: HashMap<u64, Object>,
    type_registry: Arc<RwLock<TypeRegistry>>,
}

impl ObjectHeap {
    pub fn new(type_registry: Arc<RwLock<TypeRegistry>>) -> Self {
        Self {
            objects: HashMap::new(),
            type_registry,
        }
    }

    pub fn allocate_script(&mut self, type_id: TypeId) -> Result<u64, String> {
        let type_registry = self.type_registry.read().unwrap();
        let _type_info = type_registry
            .get_type(type_id)
            .ok_or_else(|| format!("Type {} not found", type_id))?;

        drop(type_registry);

        let object = Object::new_script(type_id);
        let object_id = object.object_id();

        self.objects.insert(object_id, object);
        Ok(object_id)
    }

    pub fn allocate_rust<T: Any + Send + Sync>(
        &mut self,
        type_id: TypeId,
        rust_instance: T,
    ) -> Result<u64, String> {
        let type_registry = self.type_registry.read().unwrap();
        let _type_info = type_registry
            .get_type(type_id)
            .ok_or_else(|| format!("Type {} not found", type_id))?;

        drop(type_registry);

        let object = Object::new_rust(type_id, rust_instance);
        let object_id = object.object_id();

        self.objects.insert(object_id, object);
        Ok(object_id)
    }

    pub fn get_object(&self, object_id: u64) -> Option<&Object> {
        self.objects.get(&object_id)
    }

    pub fn get_object_mut(&mut self, object_id: u64) -> Option<&mut Object> {
        self.objects.get_mut(&object_id)
    }

    pub fn release_object(&mut self, object_id: u64) -> bool {
        if let Some(object) = self.objects.get(&object_id) {
            if object.release() {
                self.objects.remove(&object_id);
                return true;
            }
        }
        false
    }

    pub fn add_ref(&self, object_id: u64) {
        if let Some(object) = self.objects.get(&object_id) {
            object.add_ref();
        }
    }

    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    pub fn collect_garbage(&mut self, root_handles: &[u64]) {
        let mut reachable = std::collections::HashSet::new();
        let mut to_visit = root_handles.to_vec();

        while let Some(handle) = to_visit.pop() {
            if reachable.insert(handle) {
                if let Some(object) = self.objects.get(&handle) {
                    for value in object.properties().values() {
                        if let ScriptValue::ObjectHandle(child_handle) = value {
                            to_visit.push(*child_handle);
                        }
                    }
                }
            }
        }

        self.objects.retain(|id, _| reachable.contains(id));
    }
}
