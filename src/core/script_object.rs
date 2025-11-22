//! ScriptObject - asIScriptObject / CScriptObject implementation
//!
//! This module provides:
//! - `ScriptObject`: The actual storage for script-defined objects (like CScriptObject in C++)
//! - `ScriptObjectBehaviourIds`: IDs for registered ScriptObject behaviours
//! - `register_script_object_behaviours()`: Registers behaviours for script objects
//!
//! ## Architecture
//!
//! `ScriptObject` is equivalent to `CScriptObject` in C++ AngelScript.
//! It holds the actual data for script-defined class instances:
//! - Properties (HashMap<String, ScriptValue>)
//! - Reference count
//! - GC flag
//! - Type information
//!
//! ScriptObject instances are stored directly on the ObjectHeap as `Box<dyn Any>`.
//! Application-registered types are also stored directly as their concrete types.

use crate::core::types::{BehaviourType, FunctionId, ScriptValue, TypeFlags, TypeId};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

// ============================================================================
// ScriptObject - CScriptObject equivalent
// ============================================================================

/// Script object instance - equivalent to CScriptObject in C++ AngelScript
///
/// This struct holds the actual data for script-defined class instances.
/// It is stored directly on the ObjectHeap as `Box<dyn Any + Send + Sync>`.
///
/// For application-registered types, the application's Rust types are stored
/// directly on the heap instead (and must implement their own ref counting
/// via registered behaviours).
pub struct ScriptObject {
    /// The type ID of this object's class
    type_id: TypeId,
    /// Properties defined by the script class
    properties: HashMap<String, ScriptValue>,
    /// Reference count - atomically managed
    ref_count: AtomicUsize,
    /// GC flag - set during GC, cleared by AddRef/Release
    gc_flag: AtomicBool,
}

// ScriptObject must be Send + Sync to be stored as Box<dyn Any + Send + Sync>
// AtomicUsize and AtomicBool are both Send + Sync
unsafe impl Send for ScriptObject {}
unsafe impl Sync for ScriptObject {}

impl ScriptObject {
    /// Create a new script object with the given type ID
    ///
    /// Initial reference count is 1.
    pub fn new(type_id: TypeId) -> Self {
        Self {
            type_id,
            properties: HashMap::new(),
            ref_count: AtomicUsize::new(1),
            gc_flag: AtomicBool::new(false),
        }
    }

    /// Get the type ID of this object
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    // ========================================================================
    // Property access
    // ========================================================================

    /// Get a property value by name
    pub fn get_property(&self, name: &str) -> Option<&ScriptValue> {
        self.properties.get(name)
    }

    /// Set a property value by name
    pub fn set_property(&mut self, name: &str, value: ScriptValue) {
        self.properties.insert(name.to_string(), value);
    }

    /// Get all properties (read-only)
    pub fn properties(&self) -> &HashMap<String, ScriptValue> {
        &self.properties
    }

    /// Get mutable access to properties
    pub fn properties_mut(&mut self) -> &mut HashMap<String, ScriptValue> {
        &mut self.properties
    }

    /// Get property count
    pub fn property_count(&self) -> usize {
        self.properties.len()
    }

    // ========================================================================
    // Reference counting (asBEHAVE_ADDREF, asBEHAVE_RELEASE, asBEHAVE_GETREFCOUNT)
    // ========================================================================

    /// Increment reference count (asBEHAVE_ADDREF)
    ///
    /// Also clears the GC flag as per AngelScript semantics.
    pub fn add_ref(&self) {
        self.gc_flag.store(false, Ordering::SeqCst);
        self.ref_count.fetch_add(1, Ordering::SeqCst);
    }

    /// Decrement reference count (asBEHAVE_RELEASE)
    ///
    /// Also clears the GC flag as per AngelScript semantics.
    /// Returns true if refcount reached zero (object should be destroyed).
    pub fn release(&self) -> bool {
        self.gc_flag.store(false, Ordering::SeqCst);
        let old = self.ref_count.fetch_sub(1, Ordering::SeqCst);
        old <= 1
    }

    /// Get current reference count (asBEHAVE_GETREFCOUNT)
    pub fn ref_count(&self) -> usize {
        self.ref_count.load(Ordering::SeqCst)
    }

    // ========================================================================
    // GC support (asBEHAVE_SETGCFLAG, asBEHAVE_GETGCFLAG, asBEHAVE_ENUMREFS, asBEHAVE_RELEASEREFS)
    // ========================================================================

    /// Set GC flag (asBEHAVE_SETGCFLAG)
    ///
    /// The GC sets this flag during scanning. It is automatically
    /// cleared when AddRef or Release is called.
    pub fn set_gc_flag(&self, flag: bool) {
        self.gc_flag.store(flag, Ordering::SeqCst);
    }

    /// Get GC flag (asBEHAVE_GETGCFLAG)
    pub fn get_gc_flag(&self) -> bool {
        self.gc_flag.load(Ordering::SeqCst)
    }

    /// Enumerate all object references held by this object (asBEHAVE_ENUMREFS)
    ///
    /// Returns handles to all objects referenced by this object's properties.
    pub fn enumerate_references(&self) -> Vec<u64> {
        self.properties
            .values()
            .filter_map(|value| {
                if let ScriptValue::ObjectHandle(handle_id) = value {
                    if *handle_id != 0 {
                        Some(*handle_id)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    /// Release all references held by this object (asBEHAVE_RELEASEREFS)
    ///
    /// Called by the GC to break circular references.
    /// Sets all handle properties to null.
    pub fn release_all_references(&mut self) {
        for value in self.properties.values_mut() {
            if matches!(value, ScriptValue::ObjectHandle(_)) {
                *value = ScriptValue::Null;
            }
        }
    }
}

impl std::fmt::Debug for ScriptObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScriptObject")
            .field("type_id", &self.type_id)
            .field("ref_count", &self.ref_count.load(Ordering::SeqCst))
            .field("gc_flag", &self.gc_flag.load(Ordering::SeqCst))
            .field("properties", &self.properties.keys().collect::<Vec<_>>())
            .finish()
    }
}

// ============================================================================
// ScriptObjectBehaviourIds - Cached behaviour function IDs
// ============================================================================

/// The reserved type ID for the base script object type ($obj)
/// Script classes inherit behaviours from this type
pub const SCRIPT_OBJECT_TYPE_ID: TypeId = 99;

/// IDs for the registered ScriptObject behaviours
#[derive(Debug, Clone, Copy)]
pub struct ScriptObjectBehaviourIds {
    pub type_id: TypeId,
    pub construct: FunctionId,
    pub add_ref: FunctionId,
    pub release: FunctionId,
    pub op_assign: FunctionId,
    pub get_ref_count: FunctionId,
    pub set_gc_flag: FunctionId,
    pub get_gc_flag: FunctionId,
    pub enum_refs: FunctionId,
    pub release_refs: FunctionId,
}

// ============================================================================
// Behaviour Registration
// ============================================================================

/// Register ScriptObject behaviours with the engine
///
/// This is equivalent to RegisterScriptObject() in C++ AngelScript.
/// It registers the default script class behaviours that all script
/// objects inherit.
///
/// This should be called during engine initialization.
pub fn register_script_object_behaviours(
    engine: &mut crate::core::script_engine::ScriptEngine,
) -> Result<ScriptObjectBehaviourIds, String> {
    use crate::callfunc::wrappers::{wrap_method, wrap_method_const, wrap_method_void};

    // First, register the $obj type with appropriate flags
    // This is equivalent to: engine->scriptTypeBehaviours.flags = asOBJ_SCRIPT_OBJECT | asOBJ_REF | asOBJ_GC
    let type_id = engine.register_script_object_type(
        "$obj",
        TypeFlags::SCRIPT_OBJECT
            | TypeFlags::REF_TYPE
            | TypeFlags::GC_TYPE,
    )?;

    // asBEHAVE_CONSTRUCT - "void f(int&in)"
    let construct_id = engine.register_object_behaviour_with_impl(
        "$obj",
        BehaviourType::Construct,
        "void f(int &in)",
        wrap_method_void::<ScriptObject, _>(
            |_this| {
                // Construction is handled by the VM/heap allocation
            },
            "$obj::Construct",
        ),
    )?;

    // asBEHAVE_ADDREF - "void f()"
    let add_ref_id = engine.register_object_behaviour_with_impl(
        "$obj",
        BehaviourType::AddRef,
        "void f()",
        wrap_method_void::<ScriptObject, _>(
            |this| {
                this.add_ref();
            },
            "$obj::AddRef",
        ),
    )?;

    // asBEHAVE_RELEASE - "void f()"
    let release_id = engine.register_object_behaviour_with_impl(
        "$obj",
        BehaviourType::Release,
        "void f()",
        wrap_method::<ScriptObject, _, _>(|this| this.release(), "$obj::Release"),
    )?;

    // opAssign - "int &opAssign(int &in)"
    let op_assign_id = engine.register_object_method_with_impl(
        "$obj",
        "int &opAssign(int &in)",
        wrap_method::<ScriptObject, _, _>(
            |_this| {
                // Assignment is handled specially by the VM
                0i32
            },
            "$obj::opAssign",
        ),
    )?;

    // asBEHAVE_GETREFCOUNT - "int f()"
    let get_ref_count_id = engine.register_object_behaviour_with_impl(
        "$obj",
        BehaviourType::GetRefCount,
        "int f()",
        wrap_method_const::<ScriptObject, _, _>(
            |this| this.ref_count() as i32,
            "$obj::GetRefCount",
        ),
    )?;

    // asBEHAVE_SETGCFLAG - "void f()"
    let set_gc_flag_id = engine.register_object_behaviour_with_impl(
        "$obj",
        BehaviourType::SetGCFlag,
        "void f()",
        wrap_method_void::<ScriptObject, _>(
            |this| {
                this.set_gc_flag(true);
            },
            "$obj::SetGCFlag",
        ),
    )?;

    // asBEHAVE_GETGCFLAG - "bool f()"
    let get_gc_flag_id = engine.register_object_behaviour_with_impl(
        "$obj",
        BehaviourType::GetGCFlag,
        "bool f()",
        wrap_method_const::<ScriptObject, _, _>(|this| this.get_gc_flag(), "$obj::GetGCFlag"),
    )?;

    // asBEHAVE_ENUMREFS - "void f(int&in)"
    let enum_refs_id = engine.register_object_behaviour_with_impl(
        "$obj",
        BehaviourType::EnumRefs,
        "void f(int &in)",
        wrap_method_const::<ScriptObject, _, _>(
            |this| this.enumerate_references(),
            "$obj::EnumRefs",
        ),
    )?;

    // asBEHAVE_RELEASEREFS - "void f(int&in)"
    let release_refs_id = engine.register_object_behaviour_with_impl(
        "$obj",
        BehaviourType::ReleaseRefs,
        "void f(int &in)",
        wrap_method_void::<ScriptObject, _>(
            |this| {
                this.release_all_references();
            },
            "$obj::ReleaseRefs",
        ),
    )?;

    Ok(ScriptObjectBehaviourIds {
        type_id,
        construct: construct_id,
        add_ref: add_ref_id,
        release: release_id,
        op_assign: op_assign_id,
        get_ref_count: get_ref_count_id,
        set_gc_flag: set_gc_flag_id,
        get_gc_flag: get_gc_flag_id,
        enum_refs: enum_refs_id,
        release_refs: release_refs_id,
    })
}

/// Get the behaviour FunctionId from a ScriptObjectBehaviourIds struct
pub fn get_behaviour_function_id(
    ids: &ScriptObjectBehaviourIds,
    behaviour: BehaviourType,
) -> Option<FunctionId> {
    match behaviour {
        BehaviourType::Construct => Some(ids.construct),
        BehaviourType::AddRef => Some(ids.add_ref),
        BehaviourType::Release => Some(ids.release),
        BehaviourType::GetRefCount => Some(ids.get_ref_count),
        BehaviourType::SetGCFlag => Some(ids.set_gc_flag),
        BehaviourType::GetGCFlag => Some(ids.get_gc_flag),
        BehaviourType::EnumRefs => Some(ids.enum_refs),
        BehaviourType::ReleaseRefs => Some(ids.release_refs),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_object_creation() {
        let obj = ScriptObject::new(100);
        assert_eq!(obj.type_id(), 100);
        assert_eq!(obj.ref_count(), 1);
        assert!(!obj.get_gc_flag());
    }

    #[test]
    fn test_add_ref_clears_gc_flag() {
        let obj = ScriptObject::new(100);
        obj.set_gc_flag(true);
        assert!(obj.get_gc_flag());

        obj.add_ref();
        assert!(!obj.get_gc_flag());
        assert_eq!(obj.ref_count(), 2);
    }

    #[test]
    fn test_release_clears_gc_flag() {
        let obj = ScriptObject::new(100);
        obj.add_ref(); // refcount = 2
        obj.set_gc_flag(true);

        obj.release();
        assert!(!obj.get_gc_flag());
        assert_eq!(obj.ref_count(), 1);
    }

    #[test]
    fn test_property_access() {
        let mut obj = ScriptObject::new(100);
        obj.set_property("health", ScriptValue::Int32(100));
        obj.set_property("name", ScriptValue::String("Player".to_string()));

        assert_eq!(obj.get_property("health"), Some(&ScriptValue::Int32(100)));
        assert_eq!(
            obj.get_property("name"),
            Some(&ScriptValue::String("Player".to_string()))
        );
        assert_eq!(obj.get_property("missing"), None);
    }

    #[test]
    fn test_enumerate_references() {
        let mut obj = ScriptObject::new(100);
        obj.set_property("child1", ScriptValue::ObjectHandle(42));
        obj.set_property("child2", ScriptValue::ObjectHandle(43));
        obj.set_property("value", ScriptValue::Int32(10));
        obj.set_property("null_handle", ScriptValue::ObjectHandle(0));

        let refs = obj.enumerate_references();
        assert_eq!(refs.len(), 2);
        assert!(refs.contains(&42));
        assert!(refs.contains(&43));
    }

    #[test]
    fn test_release_all_references() {
        let mut obj = ScriptObject::new(100);
        obj.set_property("child1", ScriptValue::ObjectHandle(42));
        obj.set_property("value", ScriptValue::Int32(10));

        obj.release_all_references();

        assert!(matches!(
            obj.get_property("child1"),
            Some(ScriptValue::Null)
        ));
        assert!(matches!(
            obj.get_property("value"),
            Some(ScriptValue::Int32(10))
        ));
    }
}