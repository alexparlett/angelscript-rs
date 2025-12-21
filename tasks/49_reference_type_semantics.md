# Task 49: Reference Type Semantics (NoCount, NoHandle, POD)

## Problem Statement

We have incorrectly conflated terminology and semantics for reference types:

1. **`SingleRef` is misnamed** - We use `ReferenceKind::SingleRef` for `#[angelscript(nocount)]` but "single-ref" in AngelScript means `asOBJ_NOHANDLE` (no handles allowed), not `asOBJ_NOCOUNT` (no ref counting).

2. **Missing NoHandle support** - We don't have `asOBJ_NOHANDLE` semantics at all (single-reference types where scripts can't store handles).

3. **No behavior validation** - We don't validate that types have the required behaviors (AddRef/Release for standard ref types, Release for scoped, etc.) or reject forbidden behaviors.

4. **POD lacks `Copy` enforcement** - POD types should require Rust's `Copy` trait to guarantee memcpy safety.

## AngelScript Reference Type Flags

From C++ AngelScript (`as_scriptengine.cpp`):

| Flag | Meaning |
|------|---------|
| `asOBJ_REF` | Reference type (heap allocated) |
| `asOBJ_NOCOUNT` | No reference counting - app manages memory, handles still work |
| `asOBJ_NOHANDLE` | Single-reference type - no handles allowed in script |
| `asOBJ_SCOPED` | RAII-style - destroyed at scope exit |

**Mutual Exclusions** (line 1735-1741):
- `asOBJ_NOHANDLE` excludes `asOBJ_GC`, `asOBJ_SCOPED`, `asOBJ_NOCOUNT`, `asOBJ_IMPLICIT_HANDLE`
- `asOBJ_NOCOUNT` excludes `asOBJ_GC`, `asOBJ_NOHANDLE`, `asOBJ_SCOPED`
- `asOBJ_SCOPED` excludes `asOBJ_GC`, `asOBJ_NOHANDLE`, `asOBJ_NOCOUNT`, `asOBJ_IMPLICIT_HANDLE`

## Behavior Requirements Matrix

### Reference Types

| Type Kind | AddRef | Release | Factory | Handles (`T@`) | As Parameter |
|-----------|--------|---------|---------|----------------|--------------|
| Standard | **REQUIRED** | **REQUIRED** | Optional | ✓ | ✓ |
| NoCount | **FORBIDDEN** | **FORBIDDEN** | Optional | ✓ | ✓ |
| NoHandle | **FORBIDDEN** | **FORBIDDEN** | **FORBIDDEN** | ✗ | ✗ |
| Scoped | **FORBIDDEN** | **REQUIRED** | Optional | ✗ (except returns) | ✓ |

### Value Types

| Type Kind | Constructor | Destructor | opAssign | Copy Ctor |
|-----------|-------------|------------|----------|-----------|
| POD | Optional | Optional | Optional (memcpy) | Optional (memcpy) |
| Non-POD | **REQUIRED** (at least one) | **REQUIRED** | Required if assignable | Optional |

## Detailed Semantics

### NoCount (`asOBJ_NOCOUNT`)

Application provides its own memory management that isn't based on reference counting.

```cpp
// C++ registration
engine->RegisterObjectType("ref", 0, asOBJ_REF | asOBJ_NOCOUNT);
```

**Characteristics:**
- Handles ARE allowed (`MyType@` is valid)
- No AddRef/Release calls - app manages lifetime
- Scripts can pass handles around, store them in variables
- Use case: Pooled objects, arena-allocated types, externally managed objects

**Script usage:**
```angelscript
MyNoCountType@ handle = getFromPool();  // Valid - handles work
MyNoCountType@ other = handle;          // Valid - no AddRef called
```

### NoHandle (`asOBJ_NOHANDLE`)

Single-reference type where script cannot store any extra references.

```cpp
// C++ registration
engine->RegisterObjectType("single", 0, asOBJ_REF | asOBJ_NOHANDLE);
```

**Characteristics:**
- Handles NOT allowed (`MyType@` is invalid type)
- No AddRef/Release (nothing to count)
- No factories (factories return handles)
- Cannot be used as function parameter (would create stack reference)
- Only accessible via global properties or return values

**Script usage:**
```angelscript
// GameMgr is registered as NoHandle
void foo() {
    gameMgr.doSomething();              // Valid - access via global property
    GameMgr@ handle = gameMgr;          // INVALID - can't create handle
    void bar(GameMgr@ g) { }            // INVALID - can't be parameter
}
```

**Use case:** Singleton managers, game state objects that app controls completely.

### Scoped

RAII-style reference type destroyed at scope exit.

**Characteristics:**
- No handles in script (except return values from app functions)
- Must have Release behavior (called at scope exit)
- Must NOT have AddRef
- Use case: File handles, locks, transactions

### POD Value Types

Plain Old Data - can be safely memcpy'd.

**Rust requirement:** Must be `Copy` trait to guarantee:
- No `Drop` implementation that could be skipped
- Bitwise copy is safe (no internal pointers)

**Characteristics:**
- Constructor optional (can zero-initialize)
- Destructor optional (no cleanup needed)
- opAssign optional (memcpy used if absent)
- Copy constructor optional (memcpy used if absent)

## Implementation Plan

### Phase 1: Core Type System Renaming

**File:** `angelscript-core/src/type_def.rs`

```rust
// BEFORE
pub enum ReferenceKind {
    Standard,
    Scoped,
    SingleRef,  // WRONG NAME
    GenericHandle,
}

// AFTER
pub enum ReferenceKind {
    Standard,
    Scoped,
    NoCount,    // Renamed from SingleRef
    NoHandle,   // NEW
    GenericHandle,
}
```

Update methods:
```rust
impl TypeKind {
    // Rename single_ref() -> no_count()
    pub fn no_count() -> Self {
        TypeKind::Reference { kind: ReferenceKind::NoCount }
    }

    // NEW
    pub fn no_handle() -> Self {
        TypeKind::Reference { kind: ReferenceKind::NoHandle }
    }
}
```

### Phase 2: Add Query Methods

**File:** `angelscript-core/src/type_def.rs`

```rust
impl ReferenceKind {
    /// Whether this reference kind supports handle types (`T@`)
    pub fn supports_handles(&self) -> bool {
        match self {
            ReferenceKind::Standard | ReferenceKind::NoCount | ReferenceKind::GenericHandle => true,
            ReferenceKind::Scoped | ReferenceKind::NoHandle => false,
        }
    }

    /// Whether AddRef behavior is allowed
    pub fn allows_addref(&self) -> bool {
        matches!(self, ReferenceKind::Standard)
    }

    /// Whether Release behavior is allowed
    pub fn allows_release(&self) -> bool {
        matches!(self, ReferenceKind::Standard | ReferenceKind::Scoped)
    }

    /// Whether AddRef/Release are required
    pub fn requires_ref_counting(&self) -> bool {
        matches!(self, ReferenceKind::Standard)
    }

    /// Whether factories are allowed
    pub fn allows_factories(&self) -> bool {
        !matches!(self, ReferenceKind::NoHandle)
    }

    /// Whether type can be used as function parameter
    pub fn allows_as_parameter(&self) -> bool {
        !matches!(self, ReferenceKind::NoHandle)
    }
}
```

### Phase 3: Macro Support

**File:** `angelscript-macros/src/attrs.rs`

```rust
pub enum TypeKindAttr {
    Value,
    Pod,
    Reference,
    Scoped,
    NoCount,   // Keep existing
    NoHandle,  // NEW
    AsHandle,
}

// In parse_angelscript_attrs:
} else if meta.path.is_ident("nohandle") {
    result.type_kind = Some(TypeKindAttr::NoHandle);
}
```

**File:** `angelscript-macros/src/derive_any.rs`

```rust
Some(TypeKindAttr::NoCount) => quote! { ::angelscript_core::TypeKind::no_count() },
Some(TypeKindAttr::NoHandle) => quote! { ::angelscript_core::TypeKind::no_handle() },
```

### Phase 4: POD `Copy` Enforcement

**File:** `angelscript-core/src/type_def.rs`

```rust
impl TypeKind {
    /// Create a POD value type kind.
    ///
    /// # Safety
    /// T must be safe to memcpy (no internal pointers, no Drop).
    /// Prefer using this only with `Copy` types.
    pub fn pod<T: Copy>() -> Self {
        TypeKind::Value {
            size: std::mem::size_of::<T>(),
            align: std::mem::align_of::<T>(),
            is_pod: true,
        }
    }

    /// Create a POD value type without Copy bound.
    ///
    /// # Safety
    /// Caller must ensure type is safe to memcpy.
    pub unsafe fn pod_unchecked<T>() -> Self {
        TypeKind::Value {
            size: std::mem::size_of::<T>(),
            align: std::mem::align_of::<T>(),
            is_pod: true,
        }
    }
}
```

### Phase 5: Behavior Validation at Registration

**File:** `angelscript-registry/src/registry.rs`

Add validation when registering behaviors:

```rust
impl ScriptRegistry {
    /// Validate that a behavior is allowed for the given type kind.
    fn validate_behavior(
        &self,
        type_hash: TypeHash,
        behavior: BehaviorKind,
    ) -> Result<(), RegistrationError> {
        let Some(entry) = self.types.get(&type_hash) else {
            return Err(RegistrationError::TypeNotFound(type_hash));
        };

        let Some(class) = entry.as_class() else {
            return Ok(()); // Non-class types have different rules
        };

        match &class.type_kind {
            TypeKind::Reference { kind } => {
                match behavior {
                    BehaviorKind::AddRef if !kind.allows_addref() => {
                        return Err(RegistrationError::IllegalBehavior {
                            type_name: class.name.clone(),
                            behavior: "AddRef",
                            reason: format!("{:?} types cannot have AddRef", kind),
                        });
                    }
                    BehaviorKind::Release if !kind.allows_release() => {
                        return Err(RegistrationError::IllegalBehavior {
                            type_name: class.name.clone(),
                            behavior: "Release",
                            reason: format!("{:?} types cannot have Release", kind),
                        });
                    }
                    BehaviorKind::Factory | BehaviorKind::ListFactory
                        if !kind.allows_factories() => {
                        return Err(RegistrationError::IllegalBehavior {
                            type_name: class.name.clone(),
                            behavior: "Factory",
                            reason: "NoHandle types cannot have factories",
                        });
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        Ok(())
    }
}
```

### Phase 6: Module Finalization Validation

**File:** `angelscript-registry/src/registry.rs`

```rust
impl ScriptRegistry {
    /// Validate all registered types have required behaviors.
    /// Call after all registration is complete.
    pub fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        for (hash, entry) in &self.types {
            if let Some(class) = entry.as_class() {
                if let Err(e) = self.validate_type_behaviors(*hash, class) {
                    errors.push(e);
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn validate_type_behaviors(
        &self,
        type_hash: TypeHash,
        class: &Class,
    ) -> Result<(), ValidationError> {
        let behaviors = self.behaviors.get(&type_hash);

        match &class.type_kind {
            TypeKind::Reference { kind } => {
                // Standard ref types need AddRef + Release
                if kind.requires_ref_counting() {
                    let has_addref = behaviors.map_or(false, |b| b.has_addref());
                    let has_release = behaviors.map_or(false, |b| b.has_release());

                    if !has_addref || !has_release {
                        return Err(ValidationError::MissingBehavior {
                            type_name: class.name.clone(),
                            missing: "AddRef and Release required for reference types",
                        });
                    }
                }

                // Scoped types need Release
                if matches!(kind, ReferenceKind::Scoped) {
                    let has_release = behaviors.map_or(false, |b| b.has_release());
                    if !has_release {
                        return Err(ValidationError::MissingBehavior {
                            type_name: class.name.clone(),
                            missing: "Release required for scoped types",
                        });
                    }
                }
            }

            TypeKind::Value { is_pod: false, .. } => {
                // Non-POD value types need constructor + destructor
                let has_ctor = behaviors.map_or(false, |b| b.has_constructors());
                let has_dtor = behaviors.map_or(false, |b| b.has_destructor());

                if !has_ctor || !has_dtor {
                    return Err(ValidationError::MissingBehavior {
                        type_name: class.name.clone(),
                        missing: "Constructor and destructor required for non-POD value types",
                    });
                }
            }

            _ => {}
        }

        Ok(())
    }
}
```

### Phase 7: Compiler Enforcement - Handle Types

**File:** `angelscript-compiler/src/type_resolver.rs`

```rust
// In resolve() method, when applying suffixes:
for suffix in type_expr.suffixes {
    match suffix {
        TypeSuffix::Handle { is_const } => {
            // Check if type supports handles
            if let Some(entry) = self.ctx.get_type(base_hash) {
                if let Some(class) = entry.as_class() {
                    if let TypeKind::Reference { kind } = &class.type_kind {
                        if !kind.supports_handles() {
                            return Err(CompilationError::Other {
                                message: format!(
                                    "Type '{}' does not support handles ({:?} types cannot have handles)",
                                    class.name, kind
                                ),
                                span: type_expr.span,
                            });
                        }
                    }
                }
            }

            data_type.is_handle = true;
            if *is_const {
                data_type.is_handle_to_const = true;
            }
        }
    }
}
```

### Phase 8: Compiler Enforcement - Parameters

**File:** `angelscript-compiler/src/passes/registration.rs`

When registering function signatures, validate parameters:

```rust
fn validate_parameter_type(
    &self,
    param_type: &DataType,
    span: Span,
) -> Result<(), CompilationError> {
    if let Some(entry) = self.ctx.get_type(param_type.type_hash) {
        if let Some(class) = entry.as_class() {
            if let TypeKind::Reference { kind } = &class.type_kind {
                if !kind.allows_as_parameter() {
                    return Err(CompilationError::Other {
                        message: format!(
                            "Type '{}' cannot be used as a parameter (NoHandle types are single-reference)",
                            class.name
                        ),
                        span,
                    });
                }
            }
        }
    }
    Ok(())
}
```

## Files to Modify

| File | Changes |
|------|---------|
| `angelscript-core/src/type_def.rs` | Rename SingleRef→NoCount, add NoHandle, add query methods, add `Copy` bound to `pod<T>()` |
| `angelscript-macros/src/attrs.rs` | Add `NoHandle` variant, update error message |
| `angelscript-macros/src/derive_any.rs` | Map `nohandle` → `TypeKind::no_handle()` |
| `angelscript-registry/src/registry.rs` | Add `validate()` method, behavior validation |
| `angelscript-registry/src/lib.rs` | Export new error types |
| `angelscript-compiler/src/type_resolver.rs` | Validate handle support on `@` suffix |
| `angelscript-compiler/src/passes/registration.rs` | Validate NoHandle not used as parameter |
| `tests/macro_tests.rs` | Update SingleRef test, add NoHandle test |
| `docs/ffi.md` | Update documentation |

## Test Cases

### NoCount Tests
```rust
#[angelscript(nocount)]
struct PooledObject { id: u32 }

// Should work: handles allowed
// let obj: PooledObject@ = getFromPool();
// let other: PooledObject@ = obj;

// Should work: no AddRef/Release emitted
```

### NoHandle Tests
```rust
#[angelscript(nohandle)]
struct GameManager { /* ... */ }

// Should fail: GameManager@ is invalid type
// Should fail: void foo(GameManager m) - can't be parameter
// Should fail: factory registration
// Should work: access via global property
```

### POD Tests
```rust
#[derive(Copy, Clone)]
#[angelscript(pod)]
struct Vec2 { x: f32, y: f32 }

// Should work: memcpy for copy/assign
// Should work: no constructor required
// Should work: no destructor required
```

### Behavior Validation Tests
```rust
// Should fail: Standard ref without AddRef/Release
// Should fail: Scoped without Release
// Should fail: NoCount with AddRef registered
// Should fail: NoHandle with Factory registered
// Should fail: Non-POD value without constructor
```

## Migration Notes

1. **Rename only** - `SingleRef` → `NoCount` is just a rename, behavior unchanged
2. **New validation** - Existing code may fail validation if behaviors are missing
3. **POD breaking change** - Types using `TypeKind::pod<T>()` must now be `Copy`

## References

- C++ AngelScript: `reference/angelscript/source/as_scriptengine.cpp` lines 1725-1755 (flag validation)
- C++ AngelScript: `reference/angelscript/source/as_scriptengine.cpp` lines 2375-2410 (AddRef/Release validation)
- C++ AngelScript: `reference/angelscript/source/as_scriptengine.cpp` lines 3218-3250 (finalization validation)
- AngelScript docs: `reference/docs/manual/doc_adv_single_ref_type.html`
- AngelScript docs: `reference/docs/manual/doc_reg_basicref.html`
