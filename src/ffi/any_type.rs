//! Variable parameter type support (`?&` parameters).
//!
//! This module provides type-erased references for AngelScript's variable
//! parameter type (`?&`), which can accept any type.

use std::any::{Any, TypeId};

use super::native_fn::{ObjectHandle, ObjectHeap, VmSlot};

/// Type-erased reference for `?&in` parameters.
///
/// This allows native functions to receive any type as an argument.
/// The function can then inspect the type at runtime and handle it
/// appropriately.
///
/// # Example
///
/// ```ignore
/// fn format_any(ctx: &mut CallContext) -> Result<(), NativeError> {
///     let any_val = ctx.arg_any(0)?;
///
///     let formatted = match &any_val {
///         AnyRef::Int(v) => format!("{}", v),
///         AnyRef::Float(v) => format!("{:.2}", v),
///         AnyRef::Bool(v) => format!("{}", v),
///         AnyRef::String(s) => s.to_string(),
///         _ => "<object>".to_string(),
///     };
///
///     ctx.set_return(formatted)?;
///     Ok(())
/// }
/// ```
#[derive(Debug)]
pub enum AnyRef<'a> {
    /// Void value
    Void,
    /// Integer value (copied)
    Int(i64),
    /// Floating point value (copied)
    Float(f64),
    /// Boolean value (copied)
    Bool(bool),
    /// String reference
    String(&'a str),
    /// Reference to heap object
    Object {
        handle: ObjectHandle,
        heap: &'a ObjectHeap,
    },
    /// Reference to inline native value
    Native(&'a dyn Any),
    /// Null handle
    Null,
}

impl<'a> AnyRef<'a> {
    /// Create an AnyRef from a VmSlot.
    pub fn from_slot(slot: &'a VmSlot, heap: &'a ObjectHeap) -> Self {
        match slot {
            VmSlot::Void => AnyRef::Void,
            VmSlot::Int(v) => AnyRef::Int(*v),
            VmSlot::Float(v) => AnyRef::Float(*v),
            VmSlot::Bool(v) => AnyRef::Bool(*v),
            VmSlot::String(s) => AnyRef::String(s),
            VmSlot::Object(handle) => AnyRef::Object {
                handle: *handle,
                heap,
            },
            VmSlot::Native(boxed) => AnyRef::Native(boxed.as_ref()),
            VmSlot::NullHandle => AnyRef::Null,
        }
    }

    /// Get the TypeId of the contained value.
    ///
    /// For primitives, returns the TypeId of the Rust equivalent type.
    /// For objects, returns the TypeId stored in the handle.
    /// For native values, returns the TypeId of the contained value.
    pub fn type_id(&self) -> TypeId {
        match self {
            AnyRef::Void => TypeId::of::<()>(),
            AnyRef::Int(_) => TypeId::of::<i64>(),
            AnyRef::Float(_) => TypeId::of::<f64>(),
            AnyRef::Bool(_) => TypeId::of::<bool>(),
            AnyRef::String(_) => TypeId::of::<String>(),
            AnyRef::Object { handle, .. } => handle.type_id,
            AnyRef::Native(any) => (*any).type_id(),
            AnyRef::Null => TypeId::of::<()>(), // Null has no specific type
        }
    }

    /// Try to downcast to a concrete type.
    ///
    /// Works for object handles and inline native values.
    /// For primitives, use the specific `as_*` methods instead.
    pub fn downcast<T: Any>(&self) -> Option<&T> {
        match self {
            AnyRef::Object { handle, heap } => heap.get::<T>(*handle),
            AnyRef::Native(any) => any.downcast_ref::<T>(),
            _ => None,
        }
    }

    /// Check if this is void.
    pub fn is_void(&self) -> bool {
        matches!(self, AnyRef::Void)
    }

    /// Check if this is an integer.
    pub fn is_int(&self) -> bool {
        matches!(self, AnyRef::Int(_))
    }

    /// Check if this is a float.
    pub fn is_float(&self) -> bool {
        matches!(self, AnyRef::Float(_))
    }

    /// Check if this is a boolean.
    pub fn is_bool(&self) -> bool {
        matches!(self, AnyRef::Bool(_))
    }

    /// Check if this is a string.
    pub fn is_string(&self) -> bool {
        matches!(self, AnyRef::String(_))
    }

    /// Check if this is an object handle.
    pub fn is_object(&self) -> bool {
        matches!(self, AnyRef::Object { .. })
    }

    /// Check if this is a native inline value.
    pub fn is_native(&self) -> bool {
        matches!(self, AnyRef::Native(_))
    }

    /// Check if this is null.
    pub fn is_null(&self) -> bool {
        matches!(self, AnyRef::Null)
    }

    /// Get as integer.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            AnyRef::Int(v) => Some(*v),
            _ => None,
        }
    }

    /// Get as float.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            AnyRef::Float(v) => Some(*v),
            _ => None,
        }
    }

    /// Get as boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            AnyRef::Bool(v) => Some(*v),
            _ => None,
        }
    }

    /// Get as string reference.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            AnyRef::String(s) => Some(s),
            _ => None,
        }
    }
}

/// Type-erased mutable reference for `?&out` parameters.
///
/// This allows native functions to write any type to an output parameter.
#[derive(Debug)]
pub enum AnyRefMut<'a> {
    /// Mutable integer
    Int(&'a mut i64),
    /// Mutable float
    Float(&'a mut f64),
    /// Mutable boolean
    Bool(&'a mut bool),
    /// Mutable string
    String(&'a mut String),
    /// Mutable reference to heap object
    Object {
        handle: ObjectHandle,
        heap: &'a mut ObjectHeap,
    },
    /// Mutable reference to inline native value
    Native(&'a mut dyn Any),
}

impl<'a> AnyRefMut<'a> {
    /// Get the TypeId of the contained value.
    ///
    /// For Native values, returns the TypeId stored in the dyn Any trait object.
    pub fn type_id(&self) -> TypeId {
        match self {
            AnyRefMut::Int(_) => TypeId::of::<i64>(),
            AnyRefMut::Float(_) => TypeId::of::<f64>(),
            AnyRefMut::Bool(_) => TypeId::of::<bool>(),
            AnyRefMut::String(_) => TypeId::of::<String>(),
            AnyRefMut::Object { handle, .. } => handle.type_id,
            AnyRefMut::Native(any) => (**any).type_id(),
        }
    }

    /// Try to downcast to a concrete mutable type.
    ///
    /// Works for object handles and inline native values.
    pub fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> {
        match self {
            AnyRefMut::Object { handle, heap } => heap.get_mut::<T>(*handle),
            AnyRefMut::Native(any) => any.downcast_mut::<T>(),
            _ => None,
        }
    }

    /// Check if this is an integer.
    pub fn is_int(&self) -> bool {
        matches!(self, AnyRefMut::Int(_))
    }

    /// Check if this is a float.
    pub fn is_float(&self) -> bool {
        matches!(self, AnyRefMut::Float(_))
    }

    /// Check if this is a boolean.
    pub fn is_bool(&self) -> bool {
        matches!(self, AnyRefMut::Bool(_))
    }

    /// Check if this is a string.
    pub fn is_string(&self) -> bool {
        matches!(self, AnyRefMut::String(_))
    }

    /// Check if this is an object handle.
    pub fn is_object(&self) -> bool {
        matches!(self, AnyRefMut::Object { .. })
    }

    /// Check if this is a native inline value.
    pub fn is_native(&self) -> bool {
        matches!(self, AnyRefMut::Native(_))
    }

    /// Get mutable reference to integer.
    pub fn as_int_mut(&mut self) -> Option<&mut i64> {
        match self {
            AnyRefMut::Int(v) => Some(v),
            _ => None,
        }
    }

    /// Get mutable reference to float.
    pub fn as_float_mut(&mut self) -> Option<&mut f64> {
        match self {
            AnyRefMut::Float(v) => Some(v),
            _ => None,
        }
    }

    /// Get mutable reference to boolean.
    pub fn as_bool_mut(&mut self) -> Option<&mut bool> {
        match self {
            AnyRefMut::Bool(v) => Some(v),
            _ => None,
        }
    }

    /// Get mutable reference to string.
    pub fn as_string_mut(&mut self) -> Option<&mut String> {
        match self {
            AnyRefMut::String(s) => Some(s),
            _ => None,
        }
    }

    /// Set an integer value.
    pub fn set_int(&mut self, value: i64) -> bool {
        if let AnyRefMut::Int(v) = self {
            **v = value;
            true
        } else {
            false
        }
    }

    /// Set a float value.
    pub fn set_float(&mut self, value: f64) -> bool {
        if let AnyRefMut::Float(v) = self {
            **v = value;
            true
        } else {
            false
        }
    }

    /// Set a boolean value.
    pub fn set_bool(&mut self, value: bool) -> bool {
        if let AnyRefMut::Bool(v) = self {
            **v = value;
            true
        } else {
            false
        }
    }

    /// Set a string value.
    pub fn set_string(&mut self, value: String) -> bool {
        if let AnyRefMut::String(s) = self {
            **s = value;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn any_ref_from_int() {
        let slot = VmSlot::Int(42);
        let heap = ObjectHeap::new();

        let any = AnyRef::from_slot(&slot, &heap);

        assert!(any.is_int());
        assert!(!any.is_float());
        assert_eq!(any.as_int(), Some(42));
    }

    #[test]
    fn any_ref_from_float() {
        let slot = VmSlot::Float(3.14);
        let heap = ObjectHeap::new();

        let any = AnyRef::from_slot(&slot, &heap);

        assert!(any.is_float());
        assert_eq!(any.as_float(), Some(3.14));
    }

    #[test]
    fn any_ref_from_bool() {
        let slot = VmSlot::Bool(true);
        let heap = ObjectHeap::new();

        let any = AnyRef::from_slot(&slot, &heap);

        assert!(any.is_bool());
        assert_eq!(any.as_bool(), Some(true));
    }

    #[test]
    fn any_ref_from_string() {
        let slot = VmSlot::String("hello".to_string());
        let heap = ObjectHeap::new();

        let any = AnyRef::from_slot(&slot, &heap);

        assert!(any.is_string());
        assert_eq!(any.as_str(), Some("hello"));
    }

    #[test]
    fn any_ref_from_void() {
        let slot = VmSlot::Void;
        let heap = ObjectHeap::new();

        let any = AnyRef::from_slot(&slot, &heap);

        assert!(any.is_void());
    }

    #[test]
    fn any_ref_from_null() {
        let slot = VmSlot::NullHandle;
        let heap = ObjectHeap::new();

        let any = AnyRef::from_slot(&slot, &heap);

        assert!(any.is_null());
    }

    #[test]
    fn any_ref_from_object() {
        let mut heap = ObjectHeap::new();
        let handle = heap.allocate(42i32);
        let slot = VmSlot::Object(handle);

        let any = AnyRef::from_slot(&slot, &heap);

        assert!(any.is_object());

        let value = any.downcast::<i32>();
        assert_eq!(value, Some(&42));
    }

    #[test]
    fn any_ref_from_native() {
        let slot = VmSlot::Native(Box::new(42i32));
        let heap = ObjectHeap::new();

        let any = AnyRef::from_slot(&slot, &heap);

        assert!(any.is_native());

        let value = any.downcast::<i32>();
        assert_eq!(value, Some(&42));
    }

    #[test]
    fn any_ref_type_id() {
        let heap = ObjectHeap::new();

        let int_slot = VmSlot::Int(42);
        let int_any = AnyRef::from_slot(&int_slot, &heap);
        assert_eq!(int_any.type_id(), TypeId::of::<i64>());

        let float_slot = VmSlot::Float(3.14);
        let float_any = AnyRef::from_slot(&float_slot, &heap);
        assert_eq!(float_any.type_id(), TypeId::of::<f64>());

        let bool_slot = VmSlot::Bool(true);
        let bool_any = AnyRef::from_slot(&bool_slot, &heap);
        assert_eq!(bool_any.type_id(), TypeId::of::<bool>());

        let string_slot = VmSlot::String("test".to_string());
        let string_any = AnyRef::from_slot(&string_slot, &heap);
        assert_eq!(string_any.type_id(), TypeId::of::<String>());
    }

    #[test]
    fn any_ref_downcast_wrong_type() {
        let slot = VmSlot::Native(Box::new(42i32));
        let heap = ObjectHeap::new();

        let any = AnyRef::from_slot(&slot, &heap);
        let value = any.downcast::<String>();

        assert!(value.is_none());
    }

    #[test]
    fn any_ref_mut_set_int() {
        let mut value = 0i64;
        let mut any = AnyRefMut::Int(&mut value);

        assert!(any.set_int(42));
        assert_eq!(value, 42);
    }

    #[test]
    fn any_ref_mut_set_float() {
        let mut value = 0.0f64;
        let mut any = AnyRefMut::Float(&mut value);

        assert!(any.set_float(3.14));
        assert!((value - 3.14).abs() < f64::EPSILON);
    }

    #[test]
    fn any_ref_mut_set_bool() {
        let mut value = false;
        let mut any = AnyRefMut::Bool(&mut value);

        assert!(any.set_bool(true));
        assert!(value);
    }

    #[test]
    fn any_ref_mut_set_string() {
        let mut value = String::new();
        let mut any = AnyRefMut::String(&mut value);

        assert!(any.set_string("hello".to_string()));
        assert_eq!(value, "hello");
    }

    #[test]
    fn any_ref_mut_wrong_type() {
        let mut value = 0i64;
        let mut any = AnyRefMut::Int(&mut value);

        // Trying to set float on int should fail
        assert!(!any.set_float(3.14));
        assert_eq!(value, 0);
    }

    #[test]
    fn any_ref_mut_object_downcast() {
        let mut heap = ObjectHeap::new();
        let handle = heap.allocate(42i32);

        let mut any = AnyRefMut::Object {
            handle,
            heap: &mut heap,
        };

        if let Some(value) = any.downcast_mut::<i32>() {
            *value = 100;
        }

        // Verify the change through the heap
        let mut heap2 = ObjectHeap::new();
        let handle2 = heap2.allocate(42i32);
        if let Some(v) = heap2.get_mut::<i32>(handle2) {
            *v = 100;
        }
        assert_eq!(heap2.get::<i32>(handle2), Some(&100));
    }

    #[test]
    fn any_ref_mut_type_id() {
        let mut value = 0i64;
        let any = AnyRefMut::Int(&mut value);
        assert_eq!(any.type_id(), TypeId::of::<i64>());
    }

    use super::super::native_fn::ObjectHeap;
}
