# Task 01: FFI Core Infrastructure

**Status:** Not Started
**Depends On:** None
**Estimated Scope:** Core types and traits

---

## Objective

Create the foundational types, traits, and structures for the FFI registration system.

## Files to Create

- `src/ffi/mod.rs` - Module exports
- `src/ffi/types.rs` - TypeKind, Behaviors, ReferenceKind, TemplateInstanceInfo, TemplateValidation
- `src/ffi/traits.rs` - FromScript, ToScript, NativeType traits
- `src/ffi/native_fn.rs` - NativeFn, NativeCallable, CallContext
- `src/ffi/error.rs` - NativeError, ModuleError, ContextError, FfiRegistrationError
- `src/ffi/any_type.rs` - AnyRef, AnyRefMut for ?& parameters

## Design Decision: AST Primitive Reuse

**We do NOT create `TypeSpec` or `ParamDef` types.** Instead, we reuse existing AST primitives:

- `TypeExpr<'ast>` - Already has const, handle, reference modifiers, template args
- `FunctionParam<'ast>` - Already has name, type, default value
- `Ident<'ast>` - Name with source location
- `ReturnType<'ast>` - Return type (void or typed)

These types are parsed from declaration strings and stored in the Module's `Bump` arena.

## Key Types

```rust
// ════════════════════════════════════════════════════════════════════════════
// Type kind (value vs reference) - NOT type specification
// ════════════════════════════════════════════════════════════════════════════

pub enum TypeKind {
    Value { size: usize, align: usize, is_pod: bool },
    Reference { kind: ReferenceKind },
}

pub enum ReferenceKind {
    Standard,      // Full handle support with AddRef/Release
    Scoped,        // RAII-style, no handles
    SingleRef,     // App-controlled lifetime
    GenericHandle, // Type-erased container
}

// ════════════════════════════════════════════════════════════════════════════
// Type conversion traits - work with VmSlot, not TypeSpec
// ════════════════════════════════════════════════════════════════════════════

pub trait FromScript: Sized {
    fn from_vm(slot: &VmSlot) -> Result<Self, ConversionError>;
}

pub trait ToScript {
    fn to_vm(self, slot: &mut VmSlot) -> Result<(), ConversionError>;
}

// NativeType marker trait
pub trait NativeType: 'static {
    const NAME: &'static str;
}

// ════════════════════════════════════════════════════════════════════════════
// Native function handling
// ════════════════════════════════════════════════════════════════════════════

// NativeFn - type-erased callable
pub struct NativeFn {
    inner: Box<dyn NativeCallable + Send + Sync>,
}

// CallContext - bridges VM and Rust
pub struct CallContext<'vm> { ... }

// ════════════════════════════════════════════════════════════════════════════
// Template support
// ════════════════════════════════════════════════════════════════════════════

pub struct TemplateInstanceInfo {
    pub template_name: String,
    pub sub_types: Vec<DataType>,
}

pub struct TemplateValidation {
    pub is_valid: bool,
    pub error: Option<String>,
    pub needs_gc: bool,
}

// ════════════════════════════════════════════════════════════════════════════
// Behaviors - function pointers for type lifecycle
// ════════════════════════════════════════════════════════════════════════════

pub struct Behaviors {
    pub addref: Option<Box<dyn Fn(*const ()) + Send + Sync>>,
    pub release: Option<Box<dyn Fn(*const ()) + Send + Sync>>,
    pub destruct: Option<Box<dyn Fn(*mut ()) + Send + Sync>>,
}
```

## Implementation Notes

- All types should be `Send + Sync` where possible
- Use `thiserror` for error types
- Implement FromScript/ToScript for primitives: i8, i16, i32, i64, u8, u16, u32, u64, f32, f64, bool, String, &str
- NO TypeSpec or ParamDef - we use AST types directly

## Acceptance Criteria

- [ ] All core types compile and have basic tests
- [ ] FromScript/ToScript implemented for all primitive types
- [ ] NativeFn can wrap closures
- [ ] CallContext provides arg extraction and return value setting
- [ ] Error types have useful messages
- [ ] TypeKind, ReferenceKind, Behaviors defined
- [ ] TemplateInstanceInfo, TemplateValidation defined
