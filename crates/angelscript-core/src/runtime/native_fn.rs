//! Native function storage and callable trait.

use std::fmt;

use crate::TypeHash;
use crate::native_error::NativeError;

use super::CallContext;

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

/// Opaque handle for funcdef (function pointer) values.
///
/// This represents a reference to an AngelScript function that can be called.
/// The actual function pointer is managed by the VM - this is just a handle.
///
/// Used by the `#[funcdef]` macro to generate types that implement `Any`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct FuncdefHandle {
    /// Internal handle value (interpretation depends on VM implementation)
    pub handle: u64,
}

impl FuncdefHandle {
    /// Create a new funcdef handle.
    pub fn new(handle: u64) -> Self {
        Self { handle }
    }

    /// Create a null funcdef handle.
    pub fn null() -> Self {
        Self { handle: 0 }
    }

    /// Check if this is a null handle.
    pub fn is_null(&self) -> bool {
        self.handle == 0
    }
}
