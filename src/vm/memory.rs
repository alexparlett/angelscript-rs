// src/vm/memory.rs - Keep it aligned with VM needs

use crate::core::types::ScriptValue;
use crate::core::types::{TypeFlags, TypeKind, TypeRegistration};
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub trait RustTypeFactory: Send + Sync {
    fn create_instance(&self) -> HashMap<String, ScriptValue>;
}

#[derive(Clone)]
pub struct RustPropertyAccessor {
    pub getter: Option<Arc<dyn Fn(&ScriptObject) -> ScriptValue + Send + Sync>>,
    pub setter: Option<Arc<dyn Fn(&mut ScriptObject, ScriptValue) + Send + Sync>>,
}

#[derive(Clone)]
pub struct RustMethodBinding {
    pub function: Arc<dyn Fn(&mut ScriptObject, &[ScriptValue]) -> ScriptValue + Send + Sync>,
    pub param_types: Vec<u32>,
    pub return_type: u32,
}

#[derive(Clone)]
pub struct UnifiedTypeInfo {
    pub type_id: u32,
    pub name: String,
    pub type_kind: TypeKind,
    pub type_registration: TypeRegistration,
    pub flags: TypeFlags,
    pub fields: HashMap<String, FieldMetadata>,
    pub methods: HashMap<String, Vec<u32>>,
    pub vtable: Option<Vec<u32>>,
    pub base_class: Option<u32>,
    pub rust_factory: Option<Arc<Box<dyn RustTypeFactory>>>,
    pub rust_accessors: HashMap<String, RustPropertyAccessor>,
    pub rust_methods: HashMap<String, RustMethodBinding>,
}

#[derive(Debug, Clone)]
pub struct FieldMetadata {
    pub name: String,
    pub type_id: u32,
    pub is_const: bool,
}

impl UnifiedTypeInfo {
    pub fn is_value_type(&self) -> bool {
        self.flags.contains(TypeFlags::VALUE_TYPE) || self.flags.contains(TypeFlags::POD_TYPE)
    }

    pub fn is_ref_type(&self) -> bool {
        self.flags.contains(TypeFlags::REF_TYPE)
    }
}

#[derive(Clone)]
pub struct ScriptObject {
    type_id: u32,
    properties: HashMap<String, ScriptValue>,
    ref_count: Arc<RwLock<usize>>,
    rust_backing: Option<Arc<RwLock<Box<dyn Any + Send + Sync>>>>,
    object_id: u64,
    initialized_fields: HashMap<String, bool>,
}

impl ScriptObject {
    pub fn new_script_uninitialized(type_id: u32, type_info: &UnifiedTypeInfo) -> Self {
        let mut initialized_fields = HashMap::new();

        for field_name in type_info.fields.keys() {
            initialized_fields.insert(field_name.clone(), false);
        }

        Self {
            type_id,
            properties: HashMap::new(),
            ref_count: Arc::new(RwLock::new(1)),
            rust_backing: None,
            object_id: Self::generate_object_id(),
            initialized_fields,
        }
    }

    pub fn new_rust<T: Any + Send + Sync>(
        type_id: u32,
        type_info: &UnifiedTypeInfo,
        rust_instance: T,
    ) -> Self {
        let mut properties = HashMap::new();

        if let Some(factory) = &type_info.rust_factory {
            properties = factory.create_instance();
        }

        Self {
            type_id,
            properties,
            ref_count: Arc::new(RwLock::new(1)),
            rust_backing: Some(Arc::new(RwLock::new(Box::new(rust_instance)))),
            object_id: Self::generate_object_id(),
            initialized_fields: HashMap::new(),
        }
    }

    pub fn type_id(&self) -> u32 {
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

        if let Some(init_flag) = self.initialized_fields.get_mut(name) {
            *init_flag = true;
        }
    }

    pub fn is_field_initialized(&self, name: &str) -> bool {
        self.initialized_fields.get(name).copied().unwrap_or(true)
    }

    pub fn properties(&self) -> &HashMap<String, ScriptValue> {
        &self.properties
    }

    pub fn properties_mut(&mut self) -> &mut HashMap<String, ScriptValue> {
        &mut self.properties
    }

    pub fn rust_backing<T: Any>(&self) -> Option<Arc<RwLock<Box<dyn Any + Send + Sync>>>> {
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

pub struct TypeRegistry {
    types: HashMap<u32, Arc<UnifiedTypeInfo>>,
    types_by_name: HashMap<String, u32>,
    next_type_id: u32,
}

impl TypeRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            types: HashMap::new(),
            types_by_name: HashMap::new(),
            next_type_id: 100,
        };

        registry.register_primitives();
        registry
    }

    fn register_primitives(&mut self) {
        let primitives = vec![
            ("void", 0),
            ("bool", 1),
            ("int8", 2),
            ("int16", 3),
            ("int", 4),
            ("int64", 5),
            ("uint8", 6),
            ("uint16", 7),
            ("uint", 8),
            ("uint64", 9),
            ("float", 10),
            ("double", 11),
            ("string", 12),
        ];

        for (name, type_id) in primitives {
            let type_info = UnifiedTypeInfo {
                type_id,
                name: name.to_string(),
                fields: HashMap::new(),
                methods: HashMap::new(),
                vtable: None,
                base_class: None,
                flags: TypeFlags::POD_TYPE | TypeFlags::VALUE_TYPE,
                rust_factory: None,
                rust_accessors: HashMap::new(),
                rust_methods: HashMap::new(),
                type_registration: TypeRegistration::Script,
                type_kind: TypeKind::Primitive,
            };

            self.types.insert(type_id, Arc::new(type_info));
            self.types_by_name.insert(name.to_string(), type_id);
        }
    }

    pub fn register_script_type(&mut self, mut type_info: UnifiedTypeInfo) -> u32 {
        if type_info.type_id == 0 {
            type_info.type_id = self.next_type_id;
            self.next_type_id += 1;
        }

        type_info.type_registration = TypeRegistration::Script;

        let type_id = type_info.type_id;
        let name = type_info.name.clone();

        self.types.insert(type_id, Arc::new(type_info));
        self.types_by_name.insert(name, type_id);

        type_id
    }

    pub fn register_rust_type(
        &mut self,
        name: String,
        factory: Box<dyn RustTypeFactory>,
        accessors: HashMap<String, RustPropertyAccessor>,
        methods: HashMap<String, RustMethodBinding>,
        flags: TypeFlags,
    ) -> u32 {
        let type_id = self.next_type_id;
        self.next_type_id += 1;

        let type_info = UnifiedTypeInfo {
            type_id,
            name: name.clone(),
            fields: HashMap::new(),
            methods: HashMap::new(),
            vtable: None,
            base_class: None,
            flags,
            rust_factory: Some(Arc::new(factory)),
            rust_accessors: accessors,
            rust_methods: methods,
            type_registration: TypeRegistration::Application,
            type_kind: TypeKind::Class,
        };

        self.types.insert(type_id, Arc::new(type_info));
        self.types_by_name.insert(name, type_id);

        type_id
    }

    pub fn get_type(&self, type_id: u32) -> Option<Arc<UnifiedTypeInfo>> {
        self.types.get(&type_id).cloned()
    }

    pub fn get_type_by_name(&self, name: &str) -> Option<Arc<UnifiedTypeInfo>> {
        self.types_by_name
            .get(name)
            .and_then(|id| self.types.get(id))
            .cloned()
    }
}

pub struct ObjectHeap {
    objects: HashMap<u64, ScriptObject>,
    type_registry: Arc<RwLock<TypeRegistry>>,
}

impl ObjectHeap {
    pub fn new(type_registry: Arc<RwLock<TypeRegistry>>) -> Self {
        Self {
            objects: HashMap::new(),
            type_registry,
        }
    }

    pub fn allocate_script(&mut self, type_id: u32) -> Result<u64, String> {
        let type_registry = self.type_registry.read().unwrap();
        let type_info = type_registry
            .get_type(type_id)
            .ok_or_else(|| format!("Type {} not found", type_id))?;

        let object = ScriptObject::new_script_uninitialized(type_id, &type_info);
        let object_id = object.object_id();

        self.objects.insert(object_id, object);
        Ok(object_id)
    }

    pub fn allocate_rust<T: Any + Send + Sync>(
        &mut self,
        type_id: u32,
        rust_instance: T,
    ) -> Result<u64, String> {
        let type_registry = self.type_registry.read().unwrap();
        let type_info = type_registry
            .get_type(type_id)
            .ok_or_else(|| format!("Type {} not found", type_id))?;

        let object = ScriptObject::new_rust(type_id, &type_info, rust_instance);
        let object_id = object.object_id();

        self.objects.insert(object_id, object);
        Ok(object_id)
    }

    pub fn get_object(&self, object_id: u64) -> Option<&ScriptObject> {
        self.objects.get(&object_id)
    }

    pub fn get_object_mut(&mut self, object_id: u64) -> Option<&mut ScriptObject> {
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
