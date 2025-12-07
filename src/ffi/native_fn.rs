//! Native function storage and execution context.
//!
//! This module provides the infrastructure for storing and calling native
//! Rust functions from the VM.

use std::any::{Any, TypeId};
use std::fmt;

use super::error::NativeError;
use super::traits::{FromScript, NativeType, ToScript};
use crate::semantic::types::type_def::FunctionId;

/// Type-erased native function.
///
/// This wraps any callable that implements `NativeCallable`, allowing
/// functions of different signatures to be stored uniformly.
///
/// Each NativeFn has a unique FunctionId assigned at creation time,
/// ensuring consistent IDs across all Units.
///
/// The inner callable is wrapped in Arc to support cloning for FFI registration.
pub struct NativeFn {
    /// Unique FFI function ID (assigned at creation via FunctionId::next_ffi())
    pub id: FunctionId,
    inner: std::sync::Arc<dyn NativeCallable + Send + Sync>,
}

impl NativeFn {
    /// Create a new NativeFn from a callable.
    /// Automatically assigns a unique FFI FunctionId.
    pub fn new<F>(f: F) -> Self
    where
        F: NativeCallable + Send + Sync + 'static,
    {
        Self {
            id: FunctionId::next_ffi(),
            inner: std::sync::Arc::new(f),
        }
    }

    /// Call this native function with the given context.
    pub fn call(&self, ctx: &mut CallContext) -> Result<(), NativeError> {
        self.inner.call(ctx)
    }

    /// Clone this NativeFn, sharing the same underlying callable.
    ///
    /// This creates a new NativeFn with the same FunctionId and callable,
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

/// A slot in the VM that holds a value.
///
/// This enum represents all possible values that can be stored in the VM's
/// stack or registers. It uses safe Rust constructs - no raw pointers.
///
/// Note: VmSlot does not implement Clone because Native values may not be cloneable.
/// Use `VmSlot::clone_if_possible()` for slots that don't contain Native values.
pub enum VmSlot {
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

impl VmSlot {
    /// Get a human-readable name for this slot's type.
    pub fn type_name(&self) -> &'static str {
        match self {
            VmSlot::Void => "void",
            VmSlot::Int(_) => "int",
            VmSlot::Float(_) => "float",
            VmSlot::Bool(_) => "bool",
            VmSlot::String(_) => "string",
            VmSlot::Object(_) => "object",
            VmSlot::Native(_) => "native",
            VmSlot::NullHandle => "null",
        }
    }

    /// Check if this slot is void.
    pub fn is_void(&self) -> bool {
        matches!(self, VmSlot::Void)
    }

    /// Check if this slot is null.
    pub fn is_null(&self) -> bool {
        matches!(self, VmSlot::NullHandle)
    }

    /// Clone the slot if it doesn't contain a Native value.
    ///
    /// Returns None for Native values since they may not be cloneable.
    pub fn clone_if_possible(&self) -> Option<Self> {
        match self {
            VmSlot::Void => Some(VmSlot::Void),
            VmSlot::Int(v) => Some(VmSlot::Int(*v)),
            VmSlot::Float(v) => Some(VmSlot::Float(*v)),
            VmSlot::Bool(v) => Some(VmSlot::Bool(*v)),
            VmSlot::String(s) => Some(VmSlot::String(s.clone())),
            VmSlot::Object(h) => Some(VmSlot::Object(*h)),
            VmSlot::Native(_) => None,
            VmSlot::NullHandle => Some(VmSlot::NullHandle),
        }
    }
}

impl fmt::Debug for VmSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VmSlot::Void => write!(f, "Void"),
            VmSlot::Int(v) => write!(f, "Int({})", v),
            VmSlot::Float(v) => write!(f, "Float({})", v),
            VmSlot::Bool(v) => write!(f, "Bool({})", v),
            VmSlot::String(s) => write!(f, "String({:?})", s),
            VmSlot::Object(h) => write!(f, "Object({:?})", h),
            VmSlot::Native(_) => write!(f, "Native(...)"),
            VmSlot::NullHandle => write!(f, "NullHandle"),
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
    /// Type for runtime verification
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
            && slot.generation == handle.generation && slot.value.is_some() {
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
            && slot.generation == handle.generation && slot.value.is_some() {
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
            && slot.generation == handle.generation {
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
pub struct CallContext<'vm> {
    /// VM stack/argument slots
    slots: &'vm mut [VmSlot],
    /// Index of first argument (0 for functions, 1 for methods where 0 is `this`)
    arg_offset: usize,
    /// Return value slot
    return_slot: &'vm mut VmSlot,
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
        slots: &'vm mut [VmSlot],
        arg_offset: usize,
        return_slot: &'vm mut VmSlot,
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

    /// Get a typed argument at the given index.
    pub fn arg<T: FromScript>(&self, index: usize) -> Result<T, NativeError> {
        let slot_index = self.arg_offset + index;
        if slot_index >= self.slots.len() {
            return Err(NativeError::ArgumentIndexOutOfBounds {
                index,
                count: self.arg_count(),
            });
        }
        T::from_vm(&self.slots[slot_index]).map_err(NativeError::from)
    }

    /// Get a raw reference to an argument slot.
    pub fn arg_slot(&self, index: usize) -> Result<&VmSlot, NativeError> {
        let slot_index = self.arg_offset + index;
        self.slots.get(slot_index).ok_or(NativeError::ArgumentIndexOutOfBounds {
            index,
            count: self.arg_count(),
        })
    }

    /// Get a mutable reference to an argument slot.
    pub fn arg_slot_mut(&mut self, index: usize) -> Result<&mut VmSlot, NativeError> {
        let slot_index = self.arg_offset + index;
        let count = self.arg_count();
        self.slots.get_mut(slot_index).ok_or(NativeError::ArgumentIndexOutOfBounds {
            index,
            count,
        })
    }

    /// Get `this` reference for methods (immutable).
    ///
    /// Works with both VmSlot::Object (heap) and VmSlot::Native (inline).
    pub fn this<T: NativeType>(&self) -> Result<&T, NativeError> {
        if self.arg_offset == 0 {
            return Err(NativeError::invalid_this("not a method call"));
        }

        match &self.slots[0] {
            VmSlot::Object(handle) => {
                if handle.type_id != TypeId::of::<T>() {
                    return Err(NativeError::ThisTypeMismatch {
                        expected: TypeId::of::<T>(),
                        actual: handle.type_id,
                    });
                }
                self.heap
                    .get::<T>(*handle)
                    .ok_or(NativeError::StaleHandle { index: handle.index })
            }
            VmSlot::Native(boxed) => boxed
                .downcast_ref::<T>()
                .ok_or_else(|| NativeError::invalid_this("type mismatch for inline value")),
            _ => Err(NativeError::invalid_this("expected object or native value")),
        }
    }

    /// Get `this` reference for methods (mutable).
    pub fn this_mut<T: NativeType>(&mut self) -> Result<&mut T, NativeError> {
        if self.arg_offset == 0 {
            return Err(NativeError::invalid_this("not a method call"));
        }

        match &mut self.slots[0] {
            VmSlot::Object(handle) => {
                if handle.type_id != TypeId::of::<T>() {
                    return Err(NativeError::ThisTypeMismatch {
                        expected: TypeId::of::<T>(),
                        actual: handle.type_id,
                    });
                }
                let handle_copy = *handle;
                self.heap
                    .get_mut::<T>(handle_copy)
                    .ok_or(NativeError::StaleHandle {
                        index: handle_copy.index,
                    })
            }
            VmSlot::Native(boxed) => boxed
                .downcast_mut::<T>()
                .ok_or_else(|| NativeError::invalid_this("type mismatch for inline value")),
            _ => Err(NativeError::invalid_this("expected object or native value")),
        }
    }

    /// Set the return value.
    pub fn set_return<T: ToScript>(&mut self, value: T) -> Result<(), NativeError> {
        value.to_vm(self.return_slot).map_err(NativeError::from)?;
        Ok(())
    }

    /// Set the return value from a raw slot.
    pub fn set_return_slot(&mut self, slot: VmSlot) {
        *self.return_slot = slot;
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
    fn vm_slot_type_names() {
        assert_eq!(VmSlot::Void.type_name(), "void");
        assert_eq!(VmSlot::Int(0).type_name(), "int");
        assert_eq!(VmSlot::Float(0.0).type_name(), "float");
        assert_eq!(VmSlot::Bool(false).type_name(), "bool");
        assert_eq!(VmSlot::String("".into()).type_name(), "string");
        assert_eq!(VmSlot::NullHandle.type_name(), "null");
    }

    #[test]
    fn vm_slot_is_void() {
        assert!(VmSlot::Void.is_void());
        assert!(!VmSlot::Int(0).is_void());
    }

    #[test]
    fn vm_slot_is_null() {
        assert!(VmSlot::NullHandle.is_null());
        assert!(!VmSlot::Void.is_null());
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
        let mut slots = vec![VmSlot::Int(1), VmSlot::Int(2), VmSlot::Int(3)];
        let mut ret = VmSlot::Void;
        let mut heap = ObjectHeap::new();

        let ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);
        assert_eq!(ctx.arg_count(), 3);
    }

    #[test]
    fn call_context_arg() {
        let mut slots = vec![VmSlot::Int(42), VmSlot::Bool(true)];
        let mut ret = VmSlot::Void;
        let mut heap = ObjectHeap::new();

        let ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);

        let arg0: i32 = ctx.arg(0).unwrap();
        let arg1: bool = ctx.arg(1).unwrap();

        assert_eq!(arg0, 42);
        assert!(arg1);
    }

    #[test]
    fn call_context_arg_out_of_bounds() {
        let mut slots = vec![VmSlot::Int(42)];
        let mut ret = VmSlot::Void;
        let mut heap = ObjectHeap::new();

        let ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);

        let result: Result<i32, _> = ctx.arg(5);
        assert!(matches!(
            result,
            Err(NativeError::ArgumentIndexOutOfBounds { .. })
        ));
    }

    #[test]
    fn call_context_set_return() {
        let mut slots = vec![];
        let mut ret = VmSlot::Void;
        let mut heap = ObjectHeap::new();

        let mut ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);
        ctx.set_return(42i32).unwrap();

        assert!(matches!(ret, VmSlot::Int(42)));
    }

    #[test]
    fn call_context_method_arg_offset() {
        // For methods, slot 0 is `this`, so arg_offset = 1
        let mut slots = vec![
            VmSlot::Native(Box::new(TestType { value: 10 })),
            VmSlot::Int(42),
        ];
        let mut ret = VmSlot::Void;
        let mut heap = ObjectHeap::new();

        let ctx = CallContext::new(&mut slots, 1, &mut ret, &mut heap);

        assert_eq!(ctx.arg_count(), 1);
        let arg0: i32 = ctx.arg(0).unwrap();
        assert_eq!(arg0, 42);
    }

    // Test type for this() tests
    struct TestType {
        value: i32,
    }

    impl NativeType for TestType {
        const NAME: &'static str = "TestType";
    }

    #[test]
    fn call_context_this_inline() {
        let mut slots = vec![
            VmSlot::Native(Box::new(TestType { value: 42 })),
            VmSlot::Int(1),
        ];
        let mut ret = VmSlot::Void;
        let mut heap = ObjectHeap::new();

        let ctx = CallContext::new(&mut slots, 1, &mut ret, &mut heap);

        let this = ctx.this::<TestType>().unwrap();
        assert_eq!(this.value, 42);
    }

    #[test]
    fn call_context_this_mut_inline() {
        let mut slots = vec![
            VmSlot::Native(Box::new(TestType { value: 42 })),
            VmSlot::Int(1),
        ];
        let mut ret = VmSlot::Void;
        let mut heap = ObjectHeap::new();

        let mut ctx = CallContext::new(&mut slots, 1, &mut ret, &mut heap);

        let this = ctx.this_mut::<TestType>().unwrap();
        this.value = 100;

        // Verify the change
        let ctx2 = CallContext::new(&mut slots, 1, &mut ret, &mut heap);
        let this2 = ctx2.this::<TestType>().unwrap();
        assert_eq!(this2.value, 100);
    }

    #[test]
    fn call_context_this_not_method() {
        let mut slots = vec![VmSlot::Int(42)];
        let mut ret = VmSlot::Void;
        let mut heap = ObjectHeap::new();

        let ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);

        let result = ctx.this::<TestType>();
        assert!(matches!(result, Err(NativeError::InvalidThis { .. })));
    }

    #[test]
    fn native_fn_call() {
        let native = NativeFn::new(|ctx: &mut CallContext| {
            let a: i32 = ctx.arg(0)?;
            let b: i32 = ctx.arg(1)?;
            ctx.set_return(a + b)?;
            Ok(())
        });

        let mut slots = vec![VmSlot::Int(10), VmSlot::Int(20)];
        let mut ret = VmSlot::Void;
        let mut heap = ObjectHeap::new();

        let mut ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);
        native.call(&mut ctx).unwrap();

        assert!(matches!(ret, VmSlot::Int(30)));
    }

    // Additional tests for better coverage

    #[test]
    fn vm_slot_debug() {
        // Test Debug trait for VmSlot
        let void = format!("{:?}", VmSlot::Void);
        assert!(void.contains("Void"));

        let int = format!("{:?}", VmSlot::Int(42));
        assert!(int.contains("42"));

        let float = format!("{:?}", VmSlot::Float(3.14));
        assert!(float.contains("3.14"));

        let bool_slot = format!("{:?}", VmSlot::Bool(true));
        assert!(bool_slot.contains("true"));

        let string = format!("{:?}", VmSlot::String("test".into()));
        assert!(string.contains("test"));

        let null = format!("{:?}", VmSlot::NullHandle);
        assert!(null.contains("NullHandle"));

        let native = format!("{:?}", VmSlot::Native(Box::new(42i32)));
        assert!(native.contains("Native"));
    }

    #[test]
    fn vm_slot_object_type_name() {
        let mut heap = ObjectHeap::new();
        let handle = heap.allocate(42i32);
        let slot = VmSlot::Object(handle);
        assert_eq!(slot.type_name(), "object");
    }

    #[test]
    fn vm_slot_native_type_name() {
        let slot = VmSlot::Native(Box::new(42i32));
        assert_eq!(slot.type_name(), "native");
    }

    #[test]
    fn vm_slot_clone_if_possible() {
        // Can clone primitives
        assert!(VmSlot::Void.clone_if_possible().is_some());
        assert!(VmSlot::Int(42).clone_if_possible().is_some());
        assert!(VmSlot::Float(3.14).clone_if_possible().is_some());
        assert!(VmSlot::Bool(true).clone_if_possible().is_some());
        assert!(VmSlot::String("test".into()).clone_if_possible().is_some());
        assert!(VmSlot::NullHandle.clone_if_possible().is_some());

        // Cannot clone Native
        assert!(VmSlot::Native(Box::new(42i32)).clone_if_possible().is_none());

        // Can clone Object handle
        let mut heap = ObjectHeap::new();
        let handle = heap.allocate(42i32);
        assert!(VmSlot::Object(handle).clone_if_possible().is_some());
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
        assert_eq!(format!("{:?}", heap), "ObjectHeap { slot_count: 0, free_count: 0 }");
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
        let mut slots = vec![VmSlot::Int(1), VmSlot::Int(2)];
        let mut ret = VmSlot::Void;
        let mut heap = ObjectHeap::new();

        let ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);
        let debug = format!("{:?}", ctx);
        assert!(debug.contains("CallContext"));
        assert!(debug.contains("arg_count"));
    }

    #[test]
    fn call_context_arg_slot() {
        let mut slots = vec![VmSlot::Int(42), VmSlot::String("test".into())];
        let mut ret = VmSlot::Void;
        let mut heap = ObjectHeap::new();

        let ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);

        let slot0 = ctx.arg_slot(0).unwrap();
        assert!(matches!(slot0, VmSlot::Int(42)));

        let slot1 = ctx.arg_slot(1).unwrap();
        assert!(matches!(slot1, VmSlot::String(_)));
    }

    #[test]
    fn call_context_arg_slot_out_of_bounds() {
        let mut slots = vec![VmSlot::Int(42)];
        let mut ret = VmSlot::Void;
        let mut heap = ObjectHeap::new();

        let ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);
        assert!(ctx.arg_slot(5).is_err());
    }

    #[test]
    fn call_context_arg_slot_mut() {
        let mut slots = vec![VmSlot::Int(42)];
        let mut ret = VmSlot::Void;
        let mut heap = ObjectHeap::new();

        let mut ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);
        let slot = ctx.arg_slot_mut(0).unwrap();
        *slot = VmSlot::Int(100);

        assert!(matches!(slots[0], VmSlot::Int(100)));
    }

    #[test]
    fn call_context_arg_slot_mut_out_of_bounds() {
        let mut slots = vec![VmSlot::Int(42)];
        let mut ret = VmSlot::Void;
        let mut heap = ObjectHeap::new();

        let mut ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);
        assert!(ctx.arg_slot_mut(5).is_err());
    }

    #[test]
    fn call_context_set_return_slot() {
        let mut slots = vec![];
        let mut ret = VmSlot::Void;
        let mut heap = ObjectHeap::new();

        let mut ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);
        ctx.set_return_slot(VmSlot::String("result".into()));

        assert!(matches!(ret, VmSlot::String(_)));
    }

    #[test]
    fn call_context_heap_access() {
        let mut slots = vec![];
        let mut ret = VmSlot::Void;
        let mut heap = ObjectHeap::new();
        let handle = heap.allocate(42i32);

        let ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);
        assert_eq!(ctx.heap().get::<i32>(handle), Some(&42));
    }

    #[test]
    fn call_context_heap_mut_access() {
        let mut slots = vec![];
        let mut ret = VmSlot::Void;
        let mut heap = ObjectHeap::new();

        let mut ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);
        let handle = ctx.heap_mut().allocate(42i32);

        assert_eq!(ctx.heap().get::<i32>(handle), Some(&42));
    }

    #[test]
    fn call_context_this_from_heap_object() {
        let mut slots = vec![VmSlot::Void, VmSlot::Int(1)]; // placeholder for this
        let mut ret = VmSlot::Void;
        let mut heap = ObjectHeap::new();

        // Allocate object on heap
        let handle = heap.allocate(TestType { value: 99 });
        slots[0] = VmSlot::Object(handle);

        let ctx = CallContext::new(&mut slots, 1, &mut ret, &mut heap);
        let this = ctx.this::<TestType>().unwrap();
        assert_eq!(this.value, 99);
    }

    #[test]
    fn call_context_this_mut_from_heap_object() {
        let mut slots = vec![VmSlot::Void, VmSlot::Int(1)];
        let mut ret = VmSlot::Void;
        let mut heap = ObjectHeap::new();

        let handle = heap.allocate(TestType { value: 99 });
        slots[0] = VmSlot::Object(handle);

        let mut ctx = CallContext::new(&mut slots, 1, &mut ret, &mut heap);
        let this = ctx.this_mut::<TestType>().unwrap();
        this.value = 200;

        // Verify
        assert_eq!(heap.get::<TestType>(handle).unwrap().value, 200);
    }

    #[test]
    fn call_context_this_wrong_slot_type() {
        let mut slots = vec![VmSlot::Int(42), VmSlot::Int(1)];
        let mut ret = VmSlot::Void;
        let mut heap = ObjectHeap::new();

        let ctx = CallContext::new(&mut slots, 1, &mut ret, &mut heap);
        let result = ctx.this::<TestType>();
        assert!(matches!(result, Err(NativeError::InvalidThis { .. })));
    }

    struct OtherType {
        _data: i32,
    }

    impl NativeType for OtherType {
        const NAME: &'static str = "OtherType";
    }

    #[test]
    fn call_context_this_type_mismatch_heap() {
        let mut slots = vec![VmSlot::Void, VmSlot::Int(1)];
        let mut ret = VmSlot::Void;
        let mut heap = ObjectHeap::new();

        // Allocate OtherType but try to get TestType
        let handle = heap.allocate(OtherType { _data: 42 });
        slots[0] = VmSlot::Object(handle);

        let ctx = CallContext::new(&mut slots, 1, &mut ret, &mut heap);
        let result = ctx.this::<TestType>();
        assert!(matches!(result, Err(NativeError::ThisTypeMismatch { .. })));
    }

    #[test]
    fn call_context_this_stale_handle() {
        let mut slots = vec![VmSlot::Void, VmSlot::Int(1)];
        let mut ret = VmSlot::Void;
        let mut heap = ObjectHeap::new();

        let handle = heap.allocate(TestType { value: 99 });
        slots[0] = VmSlot::Object(handle);

        // Free the object
        heap.free(handle);

        let ctx = CallContext::new(&mut slots, 1, &mut ret, &mut heap);
        let result = ctx.this::<TestType>();
        assert!(matches!(result, Err(NativeError::StaleHandle { .. })));
    }

    #[test]
    fn call_context_this_mut_stale_handle() {
        let mut slots = vec![VmSlot::Void, VmSlot::Int(1)];
        let mut ret = VmSlot::Void;
        let mut heap = ObjectHeap::new();

        let handle = heap.allocate(TestType { value: 99 });
        slots[0] = VmSlot::Object(handle);
        heap.free(handle);

        let mut ctx = CallContext::new(&mut slots, 1, &mut ret, &mut heap);
        let result = ctx.this_mut::<TestType>();
        assert!(matches!(result, Err(NativeError::StaleHandle { .. })));
    }

    #[test]
    fn call_context_this_native_type_mismatch() {
        let mut slots = vec![
            VmSlot::Native(Box::new(OtherType { _data: 42 })),
            VmSlot::Int(1),
        ];
        let mut ret = VmSlot::Void;
        let mut heap = ObjectHeap::new();

        let ctx = CallContext::new(&mut slots, 1, &mut ret, &mut heap);
        let result = ctx.this::<TestType>();
        assert!(matches!(result, Err(NativeError::InvalidThis { .. })));
    }

    #[test]
    fn call_context_this_mut_native_type_mismatch() {
        let mut slots = vec![
            VmSlot::Native(Box::new(OtherType { _data: 42 })),
            VmSlot::Int(1),
        ];
        let mut ret = VmSlot::Void;
        let mut heap = ObjectHeap::new();

        let mut ctx = CallContext::new(&mut slots, 1, &mut ret, &mut heap);
        let result = ctx.this_mut::<TestType>();
        assert!(matches!(result, Err(NativeError::InvalidThis { .. })));
    }

    #[test]
    fn call_context_this_mut_not_method() {
        let mut slots = vec![VmSlot::Int(42)];
        let mut ret = VmSlot::Void;
        let mut heap = ObjectHeap::new();

        let mut ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);
        let result = ctx.this_mut::<TestType>();
        assert!(matches!(result, Err(NativeError::InvalidThis { .. })));
    }

    #[test]
    fn call_context_this_mut_wrong_slot_type() {
        let mut slots = vec![VmSlot::Int(42), VmSlot::Int(1)];
        let mut ret = VmSlot::Void;
        let mut heap = ObjectHeap::new();

        let mut ctx = CallContext::new(&mut slots, 1, &mut ret, &mut heap);
        let result = ctx.this_mut::<TestType>();
        assert!(matches!(result, Err(NativeError::InvalidThis { .. })));
    }

    #[test]
    fn call_context_this_mut_type_mismatch_heap() {
        let mut slots = vec![VmSlot::Void, VmSlot::Int(1)];
        let mut ret = VmSlot::Void;
        let mut heap = ObjectHeap::new();

        let handle = heap.allocate(OtherType { _data: 42 });
        slots[0] = VmSlot::Object(handle);

        let mut ctx = CallContext::new(&mut slots, 1, &mut ret, &mut heap);
        let result = ctx.this_mut::<TestType>();
        assert!(matches!(result, Err(NativeError::ThisTypeMismatch { .. })));
    }

    #[test]
    fn native_fn_debug() {
        let native = NativeFn::new(|_: &mut CallContext| Ok(()));
        let debug = format!("{:?}", native);
        assert!(debug.contains("NativeFn"));
    }
}
