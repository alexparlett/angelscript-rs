//! AngelScript Registry crate.
//!
//! This crate will contain the TypeRegistry and Module types in Phase 2.
//!
//! Native function and runtime types have been moved to `angelscript-core`.

// Re-export from core for backwards compatibility during transition
pub use angelscript_core::{
    // Native function types
    NativeFn, NativeCallable, VmSlot, ObjectHandle, ObjectHeap, CallContext,
    // Error types
    ConversionError, NativeError,
    // List buffer types
    ListBuffer, TupleListBuffer, ListPattern,
    // Template types
    TemplateInstanceInfo, TemplateValidation,
    // Any type support
    AnyRef, AnyRefMut,
    // Core types
    DataType, RefModifier, TypeHash, TypeKind,
};
