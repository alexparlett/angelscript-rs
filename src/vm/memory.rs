// src/vm/memory.rs - Complete memory management system

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::compiler::bytecode::ScriptValue;
// ==================== UNIFIED TYPE SYSTEM ====================

/// Unified type information for both Rust and Script types
#[derive(Clone)]
pub struct UnifiedTypeInfo {
    pub type_id: u32,
    pub name: String,

    /// Field definitions (metadata only, no memory layout)
    pub fields: HashMap<String, FieldMetadata>,

    /// Method table (name -> function IDs)
    pub methods: HashMap<String, Vec<u32>>,

    /// Virtual method table (for inheritance)
    pub vtable: Option<Vec<u32>>,

    /// Base class type ID
    pub base_class: Option<u32>,

    /// Type flags
    pub flags: TypeFlags,

    /// For Rust types: factory function to create instances
    pub rust_factory: Option<Arc<Box<dyn RustTypeFactory>>>,

    /// For Rust types: property accessor functions
    pub rust_accessors: HashMap<String, RustPropertyAccessor>,

    /// For Rust types: method bindings
    pub rust_methods: HashMap<String, RustMethodBinding>,
}

/// Field metadata (for type checking, NOT memory layout)
#[derive(Debug, Clone)]
pub struct FieldMetadata {
    pub name: String,
    pub type_id: u32,
    pub is_const: bool,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct TypeFlags: u32 {
        const SCRIPT_TYPE = 0x0001;
        const RUST_TYPE = 0x0002;
        const VALUE_TYPE = 0x0004;
        const REF_TYPE = 0x0008;
        const POD = 0x0010;
        const HAS_DESTRUCTOR = 0x0020;
        const HAS_CONSTRUCTOR = 0x0040;
        const ABSTRACT = 0x0080;
        const FINAL = 0x0100;
    }
}

impl UnifiedTypeInfo {
    /// Check if this type should be stored as a value (inline)
    pub fn is_value_type(&self) -> bool {
        self.flags.contains(TypeFlags::VALUE_TYPE) || self.flags.contains(TypeFlags::POD)
    }

    /// Check if this type should be stored as a handle
    pub fn is_ref_type(&self) -> bool {
        self.flags.contains(TypeFlags::REF_TYPE)
    }
}

// ==================== RUST TYPE INTEGRATION ====================

/// Trait for creating instances of Rust types
pub trait RustTypeFactory: Send + Sync {
    /// Create a new instance and return its properties as HashMap
    fn create_instance(&self) -> HashMap<String, ScriptValue>;
}

/// Property accessor for Rust types
#[derive(Clone)]
pub struct RustPropertyAccessor {
    /// Getter function (reads from Rust backing, returns ScriptValue)
    pub getter: Option<Arc<dyn Fn(&ScriptObject) -> ScriptValue + Send + Sync>>,

    /// Setter function (writes to Rust backing from ScriptValue)
    pub setter: Option<Arc<dyn Fn(&mut ScriptObject, ScriptValue) + Send + Sync>>,
}

/// Method binding for Rust types
#[derive(Clone)]
pub struct RustMethodBinding {
    /// The actual Rust function to call
    pub function: Arc<dyn Fn(&mut ScriptObject, &[ScriptValue]) -> ScriptValue + Send + Sync>,

    /// Parameter types
    pub param_types: Vec<u32>,

    /// Return type
    pub return_type: u32,
}

// ==================== SCRIPT OBJECT ====================

/// A heap-allocated object using pure HashMap storage
/// Properties start UNINITIALIZED - bytecode must initialize them
#[derive(Clone)]
pub struct ScriptObject {
    /// Type ID
    type_id: u32,

    /// Properties stored as HashMap
    /// For script types: starts empty, bytecode fills it
    /// For Rust types: initialized from factory
    properties: HashMap<String, ScriptValue>,

    /// Reference count
    ref_count: Arc<RwLock<usize>>,

    /// For Rust types ONLY: optional backing Rust instance
    /// Script types never use this - they're pure HashMap
    rust_backing: Option<Arc<RwLock<Box<dyn Any + Send + Sync>>>>,

    /// Object ID (unique identifier in heap)
    object_id: u64,

    /// Initialization state (for debugging/validation)
    initialized_fields: HashMap<String, bool>,
}

impl ScriptObject {
    /// Create a new UNINITIALIZED script object
    /// Bytecode constructor will initialize fields
    pub fn new_script_uninitialized(type_id: u32, type_info: &UnifiedTypeInfo) -> Self {
        let mut initialized_fields = HashMap::new();

        // Mark all fields as uninitialized
        for field_name in type_info.fields.keys() {
            initialized_fields.insert(field_name.clone(), false);
        }

        Self {
            type_id,
            properties: HashMap::new(), // Start empty!
            ref_count: Arc::new(RwLock::new(1)),
            rust_backing: None,
            object_id: Self::generate_object_id(),
            initialized_fields,
        }
    }

    /// Create a new Rust-backed object (initialized from factory)
    pub fn new_rust<T: Any + Send + Sync>(
        type_id: u32,
        type_info: &UnifiedTypeInfo,
        rust_instance: T,
    ) -> Self {
        let mut properties = HashMap::new();

        // Initialize properties from Rust instance using factory
        if let Some(factory) = &type_info.rust_factory {
            properties = factory.create_instance();
        }

        Self {
            type_id,
            properties,
            ref_count: Arc::new(RwLock::new(1)),
            rust_backing: Some(Arc::new(RwLock::new(Box::new(rust_instance)))),
            object_id: Self::generate_object_id(),
            initialized_fields: HashMap::new(), // Rust types don't track this
        }
    }

    /// Get type ID
    pub fn type_id(&self) -> u32 {
        self.type_id
    }

    /// Get object ID
    pub fn object_id(&self) -> u64 {
        self.object_id
    }

    /// Check if this is a Rust-backed object
    pub fn is_rust_backed(&self) -> bool {
        self.rust_backing.is_some()
    }

    /// Get property value
    pub fn get_property(&self, name: &str) -> Option<&ScriptValue> {
        self.properties.get(name)
    }

    /// Set property value (called by bytecode during initialization)
    pub fn set_property(&mut self, name: &str, value: ScriptValue) {
        self.properties.insert(name.to_string(), value);

        // Mark as initialized
        if let Some(init_flag) = self.initialized_fields.get_mut(name) {
            *init_flag = true;
        }
    }

    /// Check if a field has been initialized (for validation)
    pub fn is_field_initialized(&self, name: &str) -> bool {
        self.initialized_fields.get(name).copied().unwrap_or(true)
    }

    /// Get all properties
    pub fn properties(&self) -> &HashMap<String, ScriptValue> {
        &self.properties
    }

    /// Get mutable properties
    pub fn properties_mut(&mut self) -> &mut HashMap<String, ScriptValue> {
        &mut self.properties
    }

    /// Get Rust backing (for Rust types only)
    pub fn rust_backing<T: Any>(&self) -> Option<Arc<RwLock<Box<dyn Any + Send + Sync>>>> {
        self.rust_backing.clone()
    }

    /// Add reference
    pub fn add_ref(&self) {
        let mut count = self.ref_count.write().unwrap();
        *count += 1;
    }

    /// Release reference, returns true if should be destroyed
    pub fn release(&self) -> bool {
        let mut count = self.ref_count.write().unwrap();
        *count = count.saturating_sub(1);
        *count == 0
    }

    /// Get reference count
    pub fn ref_count(&self) -> usize {
        *self.ref_count.read().unwrap()
    }

    fn generate_object_id() -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    }
}

// ==================== TYPE REGISTRY ====================

/// Central registry for all types (both script and Rust)
pub struct TypeRegistry {
    /// All registered types
    types: HashMap<u32, Arc<UnifiedTypeInfo>>,

    /// Type lookup by name
    types_by_name: HashMap<String, u32>,

    /// Next type ID
    next_type_id: u32,
}

impl TypeRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            types: HashMap::new(),
            types_by_name: HashMap::new(),
            next_type_id: 100, // Start after primitives
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
                flags: TypeFlags::POD | TypeFlags::VALUE_TYPE,
                rust_factory: None,
                rust_accessors: HashMap::new(),
                rust_methods: HashMap::new(),
            };

            self.types.insert(type_id, Arc::new(type_info));
            self.types_by_name.insert(name.to_string(), type_id);
        }
    }

    /// Register a script-defined type
    pub fn register_script_type(&mut self, mut type_info: UnifiedTypeInfo) -> u32 {
        if type_info.type_id == 0 {
            type_info.type_id = self.next_type_id;
            self.next_type_id += 1;
        }

        type_info.flags |= TypeFlags::SCRIPT_TYPE;

        let type_id = type_info.type_id;
        let name = type_info.name.clone();

        self.types.insert(type_id, Arc::new(type_info));
        self.types_by_name.insert(name, type_id);

        type_id
    }

    /// Register a Rust type with bindings
    pub fn register_rust_type(
        &mut self,
        name: String,
        factory: Box<dyn RustTypeFactory>,
        accessors: HashMap<String, RustPropertyAccessor>,
        methods: HashMap<String, RustMethodBinding>,
    ) -> u32 {
        let type_id = self.next_type_id;
        self.next_type_id += 1;

        let type_info = UnifiedTypeInfo {
            type_id,
            name: name.clone(),
            fields: HashMap::new(), // Rust types don't expose fields directly
            methods: HashMap::new(),
            vtable: None,
            base_class: None,
            flags: TypeFlags::RUST_TYPE | TypeFlags::REF_TYPE,
            rust_factory: Some(Arc::new(factory)),
            rust_accessors: accessors,
            rust_methods: methods,
        };

        self.types.insert(type_id, Arc::new(type_info));
        self.types_by_name.insert(name, type_id);

        type_id
    }

    /// Get type info by ID
    pub fn get_type(&self, type_id: u32) -> Option<Arc<UnifiedTypeInfo>> {
        self.types.get(&type_id).cloned()
    }

    /// Get type info by name
    pub fn get_type_by_name(&self, name: &str) -> Option<Arc<UnifiedTypeInfo>> {
        self.types_by_name
            .get(name)
            .and_then(|id| self.types.get(id))
            .cloned()
    }
}

// ==================== OBJECT HEAP ====================

/// Heap storage for all object instances
pub struct ObjectHeap {
    /// All allocated objects (object_id -> object)
    objects: HashMap<u64, ScriptObject>,

    /// Type registry reference
    type_registry: Arc<RwLock<TypeRegistry>>,
}

impl ObjectHeap {
    pub fn new(type_registry: Arc<RwLock<TypeRegistry>>) -> Self {
        Self {
            objects: HashMap::new(),
            type_registry,
        }
    }

    /// Allocate a new UNINITIALIZED script object
    /// The bytecode constructor will initialize it
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

    /// Allocate a new Rust-backed object
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

    /// Get object by ID
    pub fn get_object(&self, object_id: u64) -> Option<&ScriptObject> {
        self.objects.get(&object_id)
    }

    /// Get mutable object by ID
    pub fn get_object_mut(&mut self, object_id: u64) -> Option<&mut ScriptObject> {
        self.objects.get_mut(&object_id)
    }

    /// Release object (decrements refcount, deallocates if zero)
    pub fn release_object(&mut self, object_id: u64) -> bool {
        if let Some(object) = self.objects.get(&object_id) {
            if object.release() {
                self.objects.remove(&object_id);
                return true;
            }
        }
        false
    }

    /// Add reference to object
    pub fn add_ref(&self, object_id: u64) {
        if let Some(object) = self.objects.get(&object_id) {
            object.add_ref();
        }
    }

    /// Get number of allocated objects
    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    /// Garbage collection (mark and sweep)
    pub fn collect_garbage(&mut self, root_handles: &[u64]) {
        // Mark phase
        let mut reachable = std::collections::HashSet::new();
        let mut to_visit = root_handles.to_vec();

        while let Some(handle) = to_visit.pop() {
            if reachable.insert(handle) {
                if let Some(object) = self.objects.get(&handle) {
                    // Find all object handles in properties
                    for value in object.properties().values() {
                        if let ScriptValue::ObjectHandle(child_handle) = value {
                            to_visit.push(*child_handle);
                        }
                    }
                }
            }
        }

        // Sweep phase - remove unreachable objects
        self.objects.retain(|id, _| reachable.contains(id));
    }
}

// ==================== TESTS ====================

#[cfg(test)]
mod tests {
    use crate::compiler::bytecode::ScriptValue;
    use super::*;

    #[test]
    fn test_script_object_creation() {
        let type_registry = Arc::new(RwLock::new(TypeRegistry::new()));
        let mut heap = ObjectHeap::new(type_registry.clone());

        // Register a script type
        let mut type_info = UnifiedTypeInfo {
            type_id: 100,
            name: "Player".to_string(),
            fields: HashMap::new(),
            methods: HashMap::new(),
            vtable: None,
            base_class: None,
            flags: TypeFlags::SCRIPT_TYPE | TypeFlags::REF_TYPE,
            rust_factory: None,
            rust_accessors: HashMap::new(),
            rust_methods: HashMap::new(),
        };

        type_info.fields.insert(
            "health".to_string(),
            FieldMetadata {
                name: "health".to_string(),
                type_id: 4,
                is_const: false,
            },
        );

        type_registry
            .write()
            .unwrap()
            .register_script_type(type_info);

        // Allocate object
        let handle = heap.allocate_script(100).unwrap();

        // Set property
        let object = heap.get_object_mut(handle).unwrap();
        object.set_property("health", ScriptValue::Int32(100));

        // Get property
        let object = heap.get_object(handle).unwrap();
        assert_eq!(
            object.get_property("health"),
            Some(&ScriptValue::Int32(100))
        );
    }

    #[test]
    fn test_reference_counting() {
        let type_registry = Arc::new(RwLock::new(TypeRegistry::new()));
        let mut heap = ObjectHeap::new(type_registry.clone());

        let type_info = UnifiedTypeInfo {
            type_id: 100,
            name: "Test".to_string(),
            fields: HashMap::new(),
            methods: HashMap::new(),
            vtable: None,
            base_class: None,
            flags: TypeFlags::SCRIPT_TYPE | TypeFlags::REF_TYPE,
            rust_factory: None,
            rust_accessors: HashMap::new(),
            rust_methods: HashMap::new(),
        };

        type_registry
            .write()
            .unwrap()
            .register_script_type(type_info);

        let handle = heap.allocate_script(100).unwrap();

        // Initial refcount is 1
        assert_eq!(heap.get_object(handle).unwrap().ref_count(), 1);

        // Add reference
        heap.add_ref(handle);
        assert_eq!(heap.get_object(handle).unwrap().ref_count(), 2);

        // Release once - object still alive
        assert!(!heap.release_object(handle));
        assert_eq!(heap.get_object(handle).unwrap().ref_count(), 1);

        // Release again - object destroyed
        assert!(heap.release_object(handle));
        assert!(heap.get_object(handle).is_none());
    }

    #[test]
    fn test_garbage_collection() {
        let type_registry = Arc::new(RwLock::new(TypeRegistry::new()));
        let mut heap = ObjectHeap::new(type_registry.clone());

        let type_info = UnifiedTypeInfo {
            type_id: 100,
            name: "Node".to_string(),
            fields: HashMap::new(),
            methods: HashMap::new(),
            vtable: None,
            base_class: None,
            flags: TypeFlags::SCRIPT_TYPE | TypeFlags::REF_TYPE,
            rust_factory: None,
            rust_accessors: HashMap::new(),
            rust_methods: HashMap::new(),
        };

        type_registry
            .write()
            .unwrap()
            .register_script_type(type_info);

        // Create objects
        let handle1 = heap.allocate_script(100).unwrap();
        let handle2 = heap.allocate_script(100).unwrap();
        let handle3 = heap.allocate_script(100).unwrap();

        // Link them: 1 -> 2
        heap.get_object_mut(handle1)
            .unwrap()
            .set_property("next", ScriptValue::ObjectHandle(handle2));

        // handle3 is unreachable

        assert_eq!(heap.object_count(), 3);

        // Collect garbage with handle1 as root
        heap.collect_garbage(&[handle1]);

        // handle1 and handle2 survive, handle3 is collected
        assert_eq!(heap.object_count(), 2);
        assert!(heap.get_object(handle1).is_some());
        assert!(heap.get_object(handle2).is_some());
        assert!(heap.get_object(handle3).is_none());
    }
}
