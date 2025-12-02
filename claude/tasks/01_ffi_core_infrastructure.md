# Task 01: FFI Core Infrastructure

**Status:** Not Started
**Depends On:** None
**Estimated Scope:** Core types and traits

---

## Objective

Create the foundational types, traits, and structures for the FFI registration system.

## Files to Create

- `src/ffi/mod.rs` - Module exports
- `src/ffi/types.rs` - TypeSpec, ParamDef, TypeKind, Behaviors, RefModifier
- `src/ffi/traits.rs` - FromScript, ToScript, NativeType traits
- `src/ffi/native_fn.rs` - NativeFn, NativeCallable, CallContext
- `src/ffi/error.rs` - NativeError, ModuleError, ContextError
- `src/ffi/any_type.rs` - AnyRef, AnyRefMut for ?& parameters

## Key Types

```rust
// TypeSpec - AngelScript type specification
pub struct TypeSpec {
    pub type_name: String,
    pub is_const: bool,
    pub is_handle: bool,
    pub is_handle_to_const: bool,
    pub is_auto_handle: bool,
    pub ref_modifier: RefModifier,
}

// FromScript/ToScript traits for type conversion
pub trait FromScript: Sized {
    fn script_type() -> TypeSpec;
    fn from_vm(slot: &VmSlot) -> Result<Self, ConversionError>;
}

pub trait ToScript {
    fn script_type() -> TypeSpec;
    fn to_vm(self, slot: &mut VmSlot) -> Result<(), ConversionError>;
}

// NativeType marker trait
pub trait NativeType: 'static {
    const NAME: &'static str;
}

// NativeFn - type-erased callable
pub struct NativeFn {
    inner: Box<dyn NativeCallable + Send + Sync>,
}

// CallContext - bridges VM and Rust
pub struct CallContext<'vm> { ... }
```

## Implementation Notes

- All types should be `Send + Sync` where possible
- Use `thiserror` for error types
- Implement FromScript/ToScript for primitives: i8, i16, i32, i64, u8, u16, u32, u64, f32, f64, bool, String, &str

## Acceptance Criteria

- [ ] All core types compile and have basic tests
- [ ] FromScript/ToScript implemented for all primitive types
- [ ] NativeFn can wrap closures
- [ ] CallContext provides arg extraction and return value setting
- [ ] Error types have useful messages
