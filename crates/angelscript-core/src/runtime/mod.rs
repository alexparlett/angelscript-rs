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

mod call_context;
mod dynamic;
mod native_fn;
mod object_heap;

pub use call_context::CallContext;
pub use dynamic::Dynamic;
pub use native_fn::{FuncdefHandle, NativeCallable, NativeFn};
pub use object_heap::{ObjectHandle, ObjectHeap};

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use super::*;
    use crate::TypeHash;
    use crate::native_error::NativeError;

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
        assert!(matches!(result, Err(NativeError::InvalidThis { .. })));
    }

    #[test]
    fn call_context_this_no_slots() {
        let mut slots: Vec<Dynamic> = vec![];
        let mut ret = Dynamic::Void;
        let mut heap = ObjectHeap::new();

        let ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);
        let result: Result<&i32, _> = ctx.this();
        assert!(matches!(result, Err(NativeError::InvalidThis { .. })));
    }

    #[test]
    fn call_context_this_not_native() {
        let mut slots = vec![Dynamic::Int(42)];
        let mut ret = Dynamic::Void;
        let mut heap = ObjectHeap::new();

        let ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);
        let result: Result<&i32, _> = ctx.this();
        assert!(matches!(result, Err(NativeError::InvalidThis { .. })));
    }

    #[test]
    fn dynamic_debug() {
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
        let heap = ObjectHeap::new();
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
        let result = ctx.arg_slot(5);
        assert!(matches!(
            result,
            Err(NativeError::ArgumentIndexOutOfBounds { index: 5, count: 1 })
        ));
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
        let result = ctx.arg_slot_mut(5);
        assert!(matches!(
            result,
            Err(NativeError::ArgumentIndexOutOfBounds { index: 5, count: 1 })
        ));
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
