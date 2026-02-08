//! Call context bridging VM and native Rust functions.

use std::any::Any;
use std::fmt;

use crate::convert::{FromDynamic, IntoDynamic};
use crate::native_error::NativeError;

use super::{Dynamic, ObjectHeap};

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
    /// This uses the `FromDynamic` trait to convert the slot value to the
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
    pub fn arg<T: FromDynamic>(&self, index: usize) -> Result<T, NativeError> {
        let slot = self.arg_slot(index)?;
        T::from_dynamic(slot).map_err(NativeError::Conversion)
    }

    /// Set the return value from a raw slot.
    pub fn set_return_slot(&mut self, slot: Dynamic) {
        *self.return_slot = slot;
    }

    /// Set a typed return value.
    ///
    /// This uses the `IntoDynamic` trait to convert the value into a
    /// Dynamic slot value.
    ///
    /// # Example
    ///
    /// ```ignore
    /// ctx.set_return(42i32);
    /// ctx.set_return(3.14f64);
    /// ctx.set_return(true);
    /// ```
    pub fn set_return<T: IntoDynamic>(&mut self, value: T) {
        *self.return_slot = value.into_dynamic();
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
