# Task 48: VTable/ITable Dispatch Architecture

## Problem Summary

Polymorphic method dispatch is broken for:
1. **Interface method calls** - concrete type unknown at compile time
2. **Virtual method calls** - derived class may override base method
3. **AddRef/Release** - must call actual object's implementation, not base

Previously, we had no vtable at all - interface-based dispatch wasn't working. The `methods: FxHashMap<String, Vec<TypeHash>>` approach couldn't handle slot-based dispatch needed for polymorphism.

---

## Key Insight: Overload vs Override Resolution

**Overload resolution is COMPILE-TIME. Override resolution is RUNTIME.**

This is how C++, Java, C#, and original AngelScript all work:

1. **At compile time**: Resolve which method *signature* is being called (overload resolution)
2. **At compile time**: Get that method's vtable slot index
3. **At runtime**: Use the slot index to look up the actual implementation in the object's vtable

References:
- [C++ VTable - Wikipedia](https://en.wikipedia.org/wiki/Virtual_method_table)
- [Java invokeinterface - OpenJDK Wiki](https://wiki.openjdk.org/display/HotSpot/InterfaceCalls)
- [C# CLR Virtual Stub Dispatch](https://github.com/dotnet/runtime/blob/main/docs/design/coreclr/botr/virtual-stub-dispatch.md)

---

## Design

### Data Structures

```rust
pub struct VTable {
    /// Slot index → method hash. Used at runtime for dispatch.
    pub slots: Vec<TypeHash>,

    /// Method name → slot indices. Used for overload resolution.
    /// Contains ALL callable methods (own + inherited).
    pub slots_by_name: FxHashMap<String, Vec<u16>>,

    /// Signature hash → slot. Internal use only during vtable construction
    /// to detect overrides (same signature = same slot).
    index: FxHashMap<u64, u16>,
}

pub struct ITable {
    /// Same structure as VTable but for interface methods.
    pub slots: Vec<TypeHash>,
    pub slots_by_name: FxHashMap<String, Vec<u16>>,
    index: FxHashMap<u64, u16>,
}

pub struct ClassEntry {
    /// Methods defined on THIS class only (not inherited).
    /// For reflection, doc gen, LSP.
    /// To filter: check func.owner_type == class_hash
    pub methods: FxHashMap<String, Vec<TypeHash>>,

    /// VTable for virtual method dispatch.
    pub vtable: VTable,

    /// ITables for each implemented interface.
    pub itables: FxHashMap<TypeHash, ITable>,
}
```

### Overload Resolution Flow (Compile Time)

For `obj.foo(42)`:

1. `vtable.slots_by_name["foo"]` → `[0, 2, 5]` (slot indices for all `foo` overloads)
2. For each slot: `vtable.slots[slot]` → method hash → `registry.get_function(hash)`
3. Check param compatibility (handles `?`, defaults, variadic)
4. Pick best match → we already have its slot index
5. Emit `CallVirtual(slot)`

### Override Detection (VTable Construction)

When building derived class vtable:

```rust
fn build_vtable(derived: &mut ClassEntry, base: &ClassEntry) {
    // Start with base vtable
    derived.vtable = base.vtable.clone();

    for own_method_hash in derived.methods.values().flatten() {
        let func = registry.get_function(own_method_hash);
        let sig_hash = compute_signature_hash(&func.name, &func.params);

        if let Some(&slot) = derived.vtable.index.get(&sig_hash) {
            // Override: same signature exists in base, replace in same slot
            derived.vtable.slots[slot] = own_method_hash;
        } else {
            // New method: add new slot
            let slot = derived.vtable.slots.len() as u16;
            derived.vtable.slots.push(own_method_hash);
            derived.vtable.index.insert(sig_hash, slot);
            derived.vtable.slots_by_name
                .entry(func.name.clone())
                .or_default()
                .push(slot);
        }
    }
}
```

### Signature Hash

Used only for override matching during vtable construction:

```rust
impl DataType {
    /// Compute signature hash including all modifiers.
    /// Includes: base type, const, handle, ref_modifier (in/out/inout)
    pub fn signature_hash(&self) -> u64;
}

impl TypeHash {
    /// Signature hash for vtable matching.
    /// Includes: method name + parameter signature hashes (with modifiers)
    /// Excludes: owner type (so Base::foo matches Derived::foo)
    /// Excludes: return type (enables covariant returns)
    pub fn from_signature(name: &str, param_sig_hashes: &[u64]) -> Self;
}
```

This ensures that `foo(int)` and `foo(int &in)` are treated as different signatures,
matching AngelScript's behavior where `inOutFlags` are part of signature comparison.

### Why This Works for `?`, Variadic, Defaults

The signature hash is NOT used at call sites. Overload resolution uses the actual function metadata:

```cpp
void foo(?in arg);           // params[0].is_auto = true
void bar(int a, int b = 10); // params[1].has_default = true
void baz(...);               // func.is_variadic = true
```

Call site `obj.foo(42)`:
1. `slots_by_name["foo"]` → slots
2. Get function, check: does `int` match `?in`? Yes (auto accepts any)
3. Emit `CallVirtual(slot)`

The param checking logic handles all special cases.

---

## Free Functions

Free functions don't need vtables - they're not polymorphic. Direct call by hash.

Overload resolution for free functions uses `registry.functions_by_name["foo"]` instead of `vtable.slots_by_name`, but the same param-matching logic.

---

## Docs/LSP: Finding Own Methods

Since `methods` contains only own methods, and `vtable.slots_by_name` contains all callable:

```rust
// Get methods defined on this class (for docs)
class.methods.get("foo")

// Get all callable methods including inherited (for autocomplete)
class.vtable.slots_by_name.get("foo")
    .map(|slots| slots.iter().map(|&s| class.vtable.slots[s]))

// Check if a method is defined here vs inherited
fn is_own_method(class_hash: TypeHash, method_hash: TypeHash) -> bool {
    let func = registry.get_function(method_hash);
    func.owner_type == Some(class_hash)
}
```

---

## Implementation Checklist

### Phase 1: Core VTable Structure

- [ ] Add `VTable` struct to `entries/class.rs`
- [ ] Add `ITable` struct to `entries/interface.rs`
- [ ] Add `TypeHash::from_signature()` to `type_hash.rs`
- [ ] Update `ClassEntry` to use `VTable`
- [ ] Update `InterfaceEntry` to use `ITable`

### Phase 2: VTable Construction

- [ ] Update `TypeCompletionPass::build_class_vtable()` to build slots, slots_by_name, index
- [ ] Update `TypeCompletionPass::build_interface_itable()` similarly
- [ ] Handle inheritance: copy base vtable, detect overrides via signature hash

### Phase 3: Call Site Compilation

- [ ] Update `find_methods()` to use `vtable.slots_by_name`
- [ ] Update method call compilation to get slot after overload resolution
- [ ] Emit `CallVirtual(slot)` for virtual class methods
- [ ] Emit `CallInterface(interface_hash, slot)` for interface methods

### Phase 4: Cleanup

- [ ] Remove old `VTableIndex` type alias
- [ ] Ensure `methods` stays own-only (no inheritance copying)
- [ ] Update tests

---

## Testing

```rust
#[test]
fn vtable_overloaded_methods_get_separate_slots() {
    // class Base { void foo(int); void foo(string); }
    // Both should get different vtable slots
}

#[test]
fn vtable_override_replaces_same_slot() {
    // class Base { void foo(int); }       // slot 0
    // class Derived : Base { void foo(int); }  // still slot 0, different impl
}

#[test]
fn vtable_new_overload_gets_new_slot() {
    // class Base { void foo(int); }            // slot 0
    // class Derived : Base { void foo(string); } // slot 1 (new overload)
}

#[test]
fn slots_by_name_contains_inherited() {
    // class Base { void foo(); }
    // class Derived : Base { void bar(); }
    // Derived.vtable.slots_by_name["foo"] should exist
}

#[test]
fn methods_contains_only_own() {
    // class Base { void foo(); }
    // class Derived : Base { void bar(); }
    // Derived.methods should NOT contain "foo"
}
```

---

## Summary

1. **`methods`** = own only (docs/LSP, filter by `owner_type` if needed)
2. **`vtable.slots`** = all callable, slot → method hash (runtime dispatch)
3. **`vtable.slots_by_name`** = all callable, name → slots (overload resolution)
4. **`vtable.index`** = sig_hash → slot (internal, override detection only)
5. **Signature hash** = name + param types, used only during vtable construction
6. **Call site** = overload resolution via slots_by_name, emit slot index, no hashing at runtime
