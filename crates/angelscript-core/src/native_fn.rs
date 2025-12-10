//! Native function storage and execution context.
//!
//! This module provides the infrastructure for storing and calling native
//! Rust functions from the VM.
//!
//! ## Key Types
//!
//! - [`Dynamic`]: Runtime value type for VM slots (primitives, objects, native values)
//! - [`NativeFn`]: Type-erased callable wrapper for FFI functions
//! - [`CallContext`]: Bridge between VM and Rust for function calls
//! - [`ObjectHeap`]: Generational arena for reference-counted objects

use std::any::{Any, TypeId};
use std::fmt;

use crate::TypeHash;
use crate::convert::{FromSlot, IntoSlot};
use crate::native_error::NativeError;

/// Type-erased native function.
///
/// This wraps any callable that implements `NativeCallable`, allowing
/// functions of different signatures to be stored uniformly.
///
/// Each NativeFn has a unique TypeHash assigned at creation time,
/// ensuring consistent IDs across all Units.
///
/// The inner callable is wrapped in Arc to support cloning for FFI registration.
pub struct NativeFn {
    /// Unique FFI function ID (assigned at creation via TypeHash::from_name("test_func"))
    pub id: TypeHash,
    inner: std::sync::Arc<dyn NativeCallable + Send + Sync>,
}

impl NativeFn {
    /// Create a new NativeFn from a callable with a specific ID.
    pub fn new<F>(id: TypeHash, f: F) -> Self
    where
        F: NativeCallable + Send + Sync + 'static,
    {
        Self {
            id,
            inner: std::sync::Arc::new(f),
        }
    }

    /// Call this native function with the given context.
    pub fn call(&self, ctx: &mut CallContext) -> Result<(), NativeError> {
        self.inner.call(ctx)
    }

    /// Clone this NativeFn, sharing the same underlying callable.
    ///
    /// This creates a new NativeFn with the same TypeHash and callable,
    /// using Arc to share the underlying implementation.
    pub fn clone_arc(&self) -> Self {
        Self {
            id: self.id,
            inner: std::sync::Arc::clone(&self.inner),
        }
    }
}

impl fmt::Debug for NativeFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeFn").finish_non_exhaustive()
    }
}

impl Clone for NativeFn {
    fn clone(&self) -> Self {
        self.clone_arc()
    }
}

/// Trait for callable native functions.
///
/// This is the core trait that all native functions must implement.
/// The `call` method receives a `CallContext` that provides access to
/// arguments and allows setting the return value.
pub trait NativeCallable {
    /// Call this function with the given context.
    fn call(&self, ctx: &mut CallContext) -> Result<(), NativeError>;
}

// Implement NativeCallable for closures that take CallContext
impl<F> NativeCallable for F
where
    F: Fn(&mut CallContext) -> Result<(), NativeError>,
{
    fn call(&self, ctx: &mut CallContext) -> Result<(), NativeError> {
        (self)(ctx)
    }
}

/// A dynamic value that can be stored in VM slots.
///
/// This enum represents all possible values that can be stored in the VM's
/// stack or registers. It uses safe Rust constructs - no raw pointers.
///
/// Similar to Rhai's `Dynamic` type, this provides a unified runtime
/// representation for all AngelScript values.
///
/// Note: Dynamic does not implement Clone because Native values may not be cloneable.
/// Use `Dynamic::clone_if_possible()` for slots that don't contain Native values.
pub enum Dynamic {
    /// Void/empty
    Void,
    /// Integer value (i8, i16, i32, i64, u8, u16, u32, u64 all stored as i64)
    Int(i64),
    /// Floating point value (f32, f64 both stored as f64)
    Float(f64),
    /// Boolean value
    Bool(bool),
    /// String value (owned)
    String(String),
    /// Handle to heap-allocated object (reference types)
    Object(ObjectHandle),
    /// Inline native value (small registered types stored directly)
    /// Uses Box<dyn Any> for type safety - no raw pointer casting
    Native(Box<dyn Any + Send + Sync>),
    /// Null handle
    NullHandle,
}

impl Dynamic {
    /// Get a human-readable name for this slot's type.
    pub fn type_name(&self) -> &'static str {
        match self {
            Dynamic::Void => "void",
            Dynamic::Int(_) => "int",
            Dynamic::Float(_) => "float",
            Dynamic::Bool(_) => "bool",
            Dynamic::String(_) => "string",
            Dynamic::Object(_) => "object",
            Dynamic::Native(_) => "native",
            Dynamic::NullHandle => "null",
        }
    }

    /// Check if this slot is void.
    pub fn is_void(&self) -> bool {
        matches!(self, Dynamic::Void)
    }

    /// Check if this slot is null.
    pub fn is_null(&self) -> bool {
        matches!(self, Dynamic::NullHandle)
    }

    /// Clone the slot if it doesn't contain a Native value.
    ///
    /// Returns None for Native values since they may not be cloneable.
    pub fn clone_if_possible(&self) -> Option<Self> {
        match self {
            Dynamic::Void => Some(Dynamic::Void),
            Dynamic::Int(v) => Some(Dynamic::Int(*v)),
            Dynamic::Float(v) => Some(Dynamic::Float(*v)),
            Dynamic::Bool(v) => Some(Dynamic::Bool(*v)),
            Dynamic::String(s) => Some(Dynamic::String(s.clone())),
            Dynamic::Object(h) => Some(Dynamic::Object(*h)),
            Dynamic::Native(_) => None,
            Dynamic::NullHandle => Some(Dynamic::NullHandle),
        }
    }
}

impl fmt::Debug for Dynamic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Dynamic::Void => write!(f, "Void"),
            Dynamic::Int(v) => write!(f, "Int({})", v),
            Dynamic::Float(v) => write!(f, "Float({})", v),
            Dynamic::Bool(v) => write!(f, "Bool({})", v),
            Dynamic::String(s) => write!(f, "String({:?})", s),
            Dynamic::Object(h) => write!(f, "Object({:?})", h),
            Dynamic::Native(_) => write!(f, "Native(...)"),
            Dynamic::NullHandle => write!(f, "NullHandle"),
        }
    }
}

impl PartialEq for Dynamic {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Dynamic::Void, Dynamic::Void) => true,
            (Dynamic::Int(a), Dynamic::Int(b)) => a == b,
            (Dynamic::Float(a), Dynamic::Float(b)) => a == b,
            (Dynamic::Bool(a), Dynamic::Bool(b)) => a == b,
            (Dynamic::String(a), Dynamic::String(b)) => a == b,
            (Dynamic::Object(a), Dynamic::Object(b)) => a == b,
            (Dynamic::NullHandle, Dynamic::NullHandle) => true,
            // Native values can't be compared for equality
            (Dynamic::Native(_), Dynamic::Native(_)) => false,
            _ => false,
        }
    }
}

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

/// Context for native function calls.
///
/// This bridges the VM and Rust, providing access to function arguments
/// and the ability to set return values.
///
/// ## Typed Argument Access
///
/// Use `arg::<T>()` for typed argument extraction with automatic conversion:
///
/// ```ignore
/// let x: i32 = ctx.arg(0)?;
/// let y: f64 = ctx.arg(1)?;
/// ```
///
/// ## Return Values
///
/// Use `set_return()` for typed return values:
///
/// ```ignore
/// ctx.set_return(x + y);
/// ```
pub struct CallContext<'vm> {
    /// VM stack/argument slots
    slots: &'vm mut [Dynamic],
    /// Index of first argument (0 for functions, 1 for methods where 0 is `this`)
    arg_offset: usize,
    /// Return value slot
    return_slot: &'vm mut Dynamic,
    /// Object heap for reference type access
    heap: &'vm mut ObjectHeap,
}

impl<'vm> CallContext<'vm> {
    /// Create a new call context.
    ///
    /// # Arguments
    ///
    /// * `slots` - The argument slots (for methods, slot 0 is `this`)
    /// * `arg_offset` - Offset to first argument (0 for functions, 1 for methods)
    /// * `return_slot` - Where to store the return value
    /// * `heap` - Object heap for reference types
    pub fn new(
        slots: &'vm mut [Dynamic],
        arg_offset: usize,
        return_slot: &'vm mut Dynamic,
        heap: &'vm mut ObjectHeap,
    ) -> Self {
        Self {
            slots,
            arg_offset,
            return_slot,
            heap,
        }
    }

    /// Get the number of arguments (excluding `this` for methods).
    pub fn arg_count(&self) -> usize {
        self.slots.len().saturating_sub(self.arg_offset)
    }

    /// Get a raw reference to an argument slot.
    pub fn arg_slot(&self, index: usize) -> Result<&Dynamic, NativeError> {
        let slot_index = self.arg_offset + index;
        self.slots
            .get(slot_index)
            .ok_or(NativeError::ArgumentIndexOutOfBounds {
                index,
                count: self.arg_count(),
            })
    }

    /// Get a mutable reference to an argument slot.
    pub fn arg_slot_mut(&mut self, index: usize) -> Result<&mut Dynamic, NativeError> {
        let slot_index = self.arg_offset + index;
        let count = self.arg_count();
        self.slots
            .get_mut(slot_index)
            .ok_or(NativeError::ArgumentIndexOutOfBounds { index, count })
    }

    /// Get a typed argument value.
    ///
    /// This uses the `FromSlot` trait to convert the slot value to the
    /// requested type. For primitives (integers, floats, bool), this
    /// performs the appropriate conversion with bounds checking.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let x: i32 = ctx.arg(0)?;
    /// let y: f64 = ctx.arg(1)?;
    /// let flag: bool = ctx.arg(2)?;
    /// ```
    pub fn arg<T: FromSlot>(&self, index: usize) -> Result<T, NativeError> {
        let slot = self.arg_slot(index)?;
        T::from_slot(slot).map_err(NativeError::Conversion)
    }

    /// Set the return value from a raw slot.
    pub fn set_return_slot(&mut self, slot: Dynamic) {
        *self.return_slot = slot;
    }

    /// Set a typed return value.
    ///
    /// This uses the `IntoSlot` trait to convert the value into a
    /// Dynamic slot value.
    ///
    /// # Example
    ///
    /// ```ignore
    /// ctx.set_return(42i32);
    /// ctx.set_return(3.14f64);
    /// ctx.set_return(true);
    /// ```
    pub fn set_return<T: IntoSlot>(&mut self, value: T) {
        *self.return_slot = value.into_slot();
    }

    /// Get an immutable reference to `this` for method calls.
    ///
    /// This extracts a reference to the receiver object from slot 0.
    /// The type must match the expected type exactly.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Slot 0 is not a Native value
    /// - The native value's type doesn't match `T`
    pub fn this<T: Any>(&self) -> Result<&T, NativeError> {
        if self.slots.is_empty() {
            return Err(NativeError::invalid_this("no slots available"));
        }

        match &self.slots[0] {
            Dynamic::Native(boxed) => boxed.downcast_ref::<T>().ok_or_else(|| {
                NativeError::invalid_this(format!(
                    "type mismatch: expected {}, got different type",
                    std::any::type_name::<T>()
                ))
            }),
            Dynamic::Object(handle) => self.heap.get::<T>(*handle).ok_or_else(|| {
                NativeError::invalid_this(format!(
                    "object type mismatch or stale handle for {}",
                    std::any::type_name::<T>()
                ))
            }),
            other => Err(NativeError::invalid_this(format!(
                "expected native or object, got {}",
                other.type_name()
            ))),
        }
    }

    /// Get a mutable reference to `this` for method calls.
    ///
    /// This extracts a mutable reference to the receiver object from slot 0.
    /// The type must match the expected type exactly.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Slot 0 is not a Native value
    /// - The native value's type doesn't match `T`
    pub fn this_mut<T: Any>(&mut self) -> Result<&mut T, NativeError> {
        if self.slots.is_empty() {
            return Err(NativeError::invalid_this("no slots available"));
        }

        // We need to handle Object handles specially since they reference the heap
        match &self.slots[0] {
            Dynamic::Object(handle) => {
                let handle = *handle;
                self.heap.get_mut::<T>(handle).ok_or_else(|| {
                    NativeError::invalid_this(format!(
                        "object type mismatch or stale handle for {}",
                        std::any::type_name::<T>()
                    ))
                })
            }
            Dynamic::Native(_) => {
                // For Native, we can access it directly
                match &mut self.slots[0] {
                    Dynamic::Native(boxed) => boxed.downcast_mut::<T>().ok_or_else(|| {
                        NativeError::invalid_this(format!(
                            "type mismatch: expected {}, got different type",
                            std::any::type_name::<T>()
                        ))
                    }),
                    _ => unreachable!(),
                }
            }
            other => Err(NativeError::invalid_this(format!(
                "expected native or object, got {}",
                other.type_name()
            ))),
        }
    }

    /// Get access to the object heap.
    pub fn heap(&self) -> &ObjectHeap {
        self.heap
    }

    /// Get mutable access to the object heap.
    pub fn heap_mut(&mut self) -> &mut ObjectHeap {
        self.heap
    }
}

impl fmt::Debug for CallContext<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CallContext")
            .field("arg_count", &self.arg_count())
            .field("arg_offset", &self.arg_offset)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dynamic_type_names() {
        assert_eq!(Dynamic::Void.type_name(), "void");
        assert_eq!(Dynamic::Int(0).type_name(), "int");
        assert_eq!(Dynamic::Float(0.0).type_name(), "float");
        assert_eq!(Dynamic::Bool(false).type_name(), "bool");
        assert_eq!(Dynamic::String("".into()).type_name(), "string");
        assert_eq!(Dynamic::NullHandle.type_name(), "null");
    }

    #[test]
    fn dynamic_is_void() {
        assert!(Dynamic::Void.is_void());
        assert!(!Dynamic::Int(0).is_void());
    }

    #[test]
    fn dynamic_is_null() {
        assert!(Dynamic::NullHandle.is_null());
        assert!(!Dynamic::Void.is_null());
    }

    #[test]
    fn object_heap_allocate_and_get() {
        let mut heap = ObjectHeap::new();
        let handle = heap.allocate(42i32);

        let value = heap.get::<i32>(handle);
        assert_eq!(value, Some(&42));
    }

    #[test]
    fn object_heap_allocate_and_get_mut() {
        let mut heap = ObjectHeap::new();
        let handle = heap.allocate(42i32);

        if let Some(value) = heap.get_mut::<i32>(handle) {
            *value = 100;
        }

        assert_eq!(heap.get::<i32>(handle), Some(&100));
    }

    #[test]
    fn object_heap_wrong_type() {
        let mut heap = ObjectHeap::new();
        let handle = heap.allocate(42i32);

        let value = heap.get::<String>(handle);
        assert!(value.is_none());
    }

    #[test]
    fn object_heap_ref_counting() {
        let mut heap = ObjectHeap::new();
        let handle = heap.allocate(42i32);

        assert_eq!(heap.ref_count(handle), Some(1));

        heap.add_ref(handle);
        assert_eq!(heap.ref_count(handle), Some(2));

        heap.release(handle);
        assert_eq!(heap.ref_count(handle), Some(1));

        heap.release(handle);
        assert_eq!(heap.ref_count(handle), None);
        assert!(heap.get::<i32>(handle).is_none());
    }

    #[test]
    fn object_heap_generational_handles() {
        let mut heap = ObjectHeap::new();
        let handle1 = heap.allocate(42i32);

        // Free the object
        heap.free(handle1);

        // Old handle should be invalid
        assert!(heap.get::<i32>(handle1).is_none());

        // Allocate new object in same slot
        let handle2 = heap.allocate(100i32);

        // New handle should work
        assert_eq!(heap.get::<i32>(handle2), Some(&100));

        // Old handle should still be invalid (different generation)
        assert!(heap.get::<i32>(handle1).is_none());
    }

    #[test]
    fn call_context_arg_count() {
        let mut slots = vec![Dynamic::Int(1), Dynamic::Int(2), Dynamic::Int(3)];
        let mut ret = Dynamic::Void;
        let mut heap = ObjectHeap::new();

        let ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);
        assert_eq!(ctx.arg_count(), 3);
    }

    #[test]
    fn call_context_method_arg_offset() {
        // For methods, slot 0 is `this`, so arg_offset = 1
        let mut slots = vec![Dynamic::Native(Box::new(42i32)), Dynamic::Int(42)];
        let mut ret = Dynamic::Void;
        let mut heap = ObjectHeap::new();

        let ctx = CallContext::new(&mut slots, 1, &mut ret, &mut heap);
        assert_eq!(ctx.arg_count(), 1);
    }

    #[test]
    fn native_fn_call() {
        let native = NativeFn::new(TypeHash::from_name("test_add"), |ctx: &mut CallContext| {
            let a: i64 = ctx.arg(0)?;
            let b: i64 = ctx.arg(1)?;
            ctx.set_return(a + b);
            Ok(())
        });

        let mut slots = vec![Dynamic::Int(10), Dynamic::Int(20)];
        let mut ret = Dynamic::Void;
        let mut heap = ObjectHeap::new();

        let mut ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);
        native.call(&mut ctx).unwrap();

        assert!(matches!(ret, Dynamic::Int(30)));
    }

    #[test]
    fn call_context_typed_arg() {
        let mut slots = vec![Dynamic::Int(42), Dynamic::Float(3.14), Dynamic::Bool(true)];
        let mut ret = Dynamic::Void;
        let mut heap = ObjectHeap::new();

        let ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);

        let x: i32 = ctx.arg(0).unwrap();
        assert_eq!(x, 42);

        let y: f64 = ctx.arg(1).unwrap();
        assert!((y - 3.14).abs() < 0.001);

        let z: bool = ctx.arg(2).unwrap();
        assert!(z);
    }

    #[test]
    fn call_context_typed_return() {
        let mut slots = vec![];
        let mut ret = Dynamic::Void;
        let mut heap = ObjectHeap::new();

        let mut ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);
        ctx.set_return(42i32);

        assert!(matches!(ret, Dynamic::Int(42)));
    }

    #[test]
    fn call_context_this_native() {
        let mut slots = vec![Dynamic::Native(Box::new(42i32)), Dynamic::Int(10)];
        let mut ret = Dynamic::Void;
        let mut heap = ObjectHeap::new();

        let ctx = CallContext::new(&mut slots, 1, &mut ret, &mut heap);
        let this: &i32 = ctx.this().unwrap();
        assert_eq!(*this, 42);
    }

    #[test]
    fn call_context_this_mut_native() {
        let mut slots = vec![Dynamic::Native(Box::new(42i32)), Dynamic::Int(10)];
        let mut ret = Dynamic::Void;
        let mut heap = ObjectHeap::new();

        let mut ctx = CallContext::new(&mut slots, 1, &mut ret, &mut heap);
        let this: &mut i32 = ctx.this_mut().unwrap();
        *this = 100;

        // Verify the change
        let ctx2 = CallContext::new(&mut slots, 1, &mut ret, &mut heap);
        let this2: &i32 = ctx2.this().unwrap();
        assert_eq!(*this2, 100);
    }

    #[test]
    fn call_context_this_object() {
        let mut heap = ObjectHeap::new();
        let handle = heap.allocate(42i32);

        let mut slots = vec![Dynamic::Object(handle), Dynamic::Int(10)];
        let mut ret = Dynamic::Void;

        let ctx = CallContext::new(&mut slots, 1, &mut ret, &mut heap);
        let this: &i32 = ctx.this().unwrap();
        assert_eq!(*this, 42);
    }

    #[test]
    fn call_context_this_mut_object() {
        let mut heap = ObjectHeap::new();
        let handle = heap.allocate(42i32);

        let mut slots = vec![Dynamic::Object(handle), Dynamic::Int(10)];
        let mut ret = Dynamic::Void;

        let mut ctx = CallContext::new(&mut slots, 1, &mut ret, &mut heap);
        let this: &mut i32 = ctx.this_mut().unwrap();
        *this = 100;

        assert_eq!(heap.get::<i32>(handle), Some(&100));
    }

    #[test]
    fn call_context_this_wrong_type() {
        let mut slots = vec![Dynamic::Native(Box::new(42i32))];
        let mut ret = Dynamic::Void;
        let mut heap = ObjectHeap::new();

        let ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);
        let result: Result<&String, _> = ctx.this();
        assert!(result.is_err());
    }

    #[test]
    fn call_context_this_no_slots() {
        let mut slots: Vec<Dynamic> = vec![];
        let mut ret = Dynamic::Void;
        let mut heap = ObjectHeap::new();

        let ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);
        let result: Result<&i32, _> = ctx.this();
        assert!(result.is_err());
    }

    #[test]
    fn call_context_this_not_native() {
        let mut slots = vec![Dynamic::Int(42)];
        let mut ret = Dynamic::Void;
        let mut heap = ObjectHeap::new();

        let ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);
        let result: Result<&i32, _> = ctx.this();
        assert!(result.is_err());
    }

    // Additional tests for better coverage

    #[test]
    fn dynamic_debug() {
        // Test Debug trait for Dynamic
        let void = format!("{:?}", Dynamic::Void);
        assert!(void.contains("Void"));

        let int = format!("{:?}", Dynamic::Int(42));
        assert!(int.contains("42"));

        let float = format!("{:?}", Dynamic::Float(3.14));
        assert!(float.contains("3.14"));

        let bool_slot = format!("{:?}", Dynamic::Bool(true));
        assert!(bool_slot.contains("true"));

        let string = format!("{:?}", Dynamic::String("test".into()));
        assert!(string.contains("test"));

        let null = format!("{:?}", Dynamic::NullHandle);
        assert!(null.contains("NullHandle"));

        let native = format!("{:?}", Dynamic::Native(Box::new(42i32)));
        assert!(native.contains("Native"));
    }

    #[test]
    fn dynamic_object_type_name() {
        let mut heap = ObjectHeap::new();
        let handle = heap.allocate(42i32);
        let slot = Dynamic::Object(handle);
        assert_eq!(slot.type_name(), "object");
    }

    #[test]
    fn dynamic_native_type_name() {
        let slot = Dynamic::Native(Box::new(42i32));
        assert_eq!(slot.type_name(), "native");
    }

    #[test]
    fn dynamic_clone_if_possible() {
        // Can clone primitives
        assert!(Dynamic::Void.clone_if_possible().is_some());
        assert!(Dynamic::Int(42).clone_if_possible().is_some());
        assert!(Dynamic::Float(3.14).clone_if_possible().is_some());
        assert!(Dynamic::Bool(true).clone_if_possible().is_some());
        assert!(Dynamic::String("test".into()).clone_if_possible().is_some());
        assert!(Dynamic::NullHandle.clone_if_possible().is_some());

        // Cannot clone Native
        assert!(
            Dynamic::Native(Box::new(42i32))
                .clone_if_possible()
                .is_none()
        );

        // Can clone Object handle
        let mut heap = ObjectHeap::new();
        let handle = heap.allocate(42i32);
        assert!(Dynamic::Object(handle).clone_if_possible().is_some());
    }

    #[test]
    fn object_handle_new() {
        let handle = ObjectHandle::new(10, 5, TypeId::of::<i32>());
        assert_eq!(handle.index, 10);
        assert_eq!(handle.generation, 5);
        assert_eq!(handle.type_id, TypeId::of::<i32>());
    }

    #[test]
    fn object_heap_default() {
        let heap = ObjectHeap::default();
        assert_eq!(
            format!("{:?}", heap),
            "ObjectHeap { slot_count: 0, free_count: 0 }"
        );
    }

    #[test]
    fn object_heap_add_ref_invalid_handle() {
        let mut heap = ObjectHeap::new();
        let fake_handle = ObjectHandle::new(999, 0, TypeId::of::<i32>());
        assert!(!heap.add_ref(fake_handle));
    }

    #[test]
    fn object_heap_release_invalid_handle() {
        let mut heap = ObjectHeap::new();
        let fake_handle = ObjectHandle::new(999, 0, TypeId::of::<i32>());
        assert!(!heap.release(fake_handle));
    }

    #[test]
    fn object_heap_ref_count_invalid_handle() {
        let mut heap = ObjectHeap::new();
        let fake_handle = ObjectHandle::new(999, 0, TypeId::of::<i32>());
        assert!(heap.ref_count(fake_handle).is_none());
    }

    #[test]
    fn object_heap_stale_add_ref() {
        let mut heap = ObjectHeap::new();
        let handle = heap.allocate(42i32);
        heap.free(handle);
        assert!(!heap.add_ref(handle));
    }

    #[test]
    fn object_heap_stale_release() {
        let mut heap = ObjectHeap::new();
        let handle = heap.allocate(42i32);
        heap.free(handle);
        assert!(!heap.release(handle));
    }

    #[test]
    fn object_heap_free_invalid_handle() {
        let mut heap = ObjectHeap::new();
        let fake_handle = ObjectHandle::new(999, 0, TypeId::of::<i32>());
        // Should not panic
        heap.free(fake_handle);
    }

    #[test]
    fn object_heap_get_mut_wrong_type() {
        let mut heap = ObjectHeap::new();
        let handle = heap.allocate(42i32);
        assert!(heap.get_mut::<String>(handle).is_none());
    }

    #[test]
    fn object_heap_get_stale_handle() {
        let mut heap = ObjectHeap::new();
        let handle = heap.allocate(42i32);
        heap.free(handle);
        assert!(heap.get::<i32>(handle).is_none());
    }

    #[test]
    fn object_heap_get_mut_stale_handle() {
        let mut heap = ObjectHeap::new();
        let handle = heap.allocate(42i32);
        heap.free(handle);
        assert!(heap.get_mut::<i32>(handle).is_none());
    }

    #[test]
    fn call_context_debug() {
        let mut slots = vec![Dynamic::Int(1), Dynamic::Int(2)];
        let mut ret = Dynamic::Void;
        let mut heap = ObjectHeap::new();

        let ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);
        let debug = format!("{:?}", ctx);
        assert!(debug.contains("CallContext"));
        assert!(debug.contains("arg_count"));
    }

    #[test]
    fn call_context_arg_slot() {
        let mut slots = vec![Dynamic::Int(42), Dynamic::String("test".into())];
        let mut ret = Dynamic::Void;
        let mut heap = ObjectHeap::new();

        let ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);

        let slot0 = ctx.arg_slot(0).unwrap();
        assert!(matches!(slot0, Dynamic::Int(42)));

        let slot1 = ctx.arg_slot(1).unwrap();
        assert!(matches!(slot1, Dynamic::String(_)));
    }

    #[test]
    fn call_context_arg_slot_out_of_bounds() {
        let mut slots = vec![Dynamic::Int(42)];
        let mut ret = Dynamic::Void;
        let mut heap = ObjectHeap::new();

        let ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);
        assert!(ctx.arg_slot(5).is_err());
    }

    #[test]
    fn call_context_arg_slot_mut() {
        let mut slots = vec![Dynamic::Int(42)];
        let mut ret = Dynamic::Void;
        let mut heap = ObjectHeap::new();

        let mut ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);
        let slot = ctx.arg_slot_mut(0).unwrap();
        *slot = Dynamic::Int(100);

        assert!(matches!(slots[0], Dynamic::Int(100)));
    }

    #[test]
    fn call_context_arg_slot_mut_out_of_bounds() {
        let mut slots = vec![Dynamic::Int(42)];
        let mut ret = Dynamic::Void;
        let mut heap = ObjectHeap::new();

        let mut ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);
        assert!(ctx.arg_slot_mut(5).is_err());
    }

    #[test]
    fn call_context_set_return_slot() {
        let mut slots = vec![];
        let mut ret = Dynamic::Void;
        let mut heap = ObjectHeap::new();

        let mut ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);
        ctx.set_return_slot(Dynamic::String("result".into()));

        assert!(matches!(ret, Dynamic::String(_)));
    }

    #[test]
    fn call_context_heap_access() {
        let mut slots = vec![];
        let mut ret = Dynamic::Void;
        let mut heap = ObjectHeap::new();
        let handle = heap.allocate(42i32);

        let ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);
        assert_eq!(ctx.heap().get::<i32>(handle), Some(&42));
    }

    #[test]
    fn call_context_heap_mut_access() {
        let mut slots = vec![];
        let mut ret = Dynamic::Void;
        let mut heap = ObjectHeap::new();

        let mut ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);
        let handle = ctx.heap_mut().allocate(42i32);

        assert_eq!(ctx.heap().get::<i32>(handle), Some(&42));
    }

    #[test]
    fn native_fn_debug() {
        let native = NativeFn::new(TypeHash::from_name("test_debug"), |_: &mut CallContext| {
            Ok(())
        });
        let debug = format!("{:?}", native);
        assert!(debug.contains("NativeFn"));
    }
}
