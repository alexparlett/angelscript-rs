# Task 47b1: Override Resolution in Inheritance

## Problem

When a derived class overrides a base class method, both methods end up in the derived class's method list, causing `AmbiguousOverload` errors.

**Error:** `AmbiguousOverload { name: "speak", candidates: "speak() and speak()" }`

**Affected Test:** `test_inheritance`

## Root Cause

In `crates/angelscript-compiler/src/passes/completion.rs:448-451`:

```rust
// Copy methods from base class
for (name, method_hash) in inherited.methods {
    class.add_method(name, method_hash);  // Just adds without checking for override
    output.methods_inherited += 1;
}
```

The `add_method` function in `crates/angelscript-core/src/entries/class.rs:165-170` simply pushes to the methods vector:

```rust
pub fn add_method(&mut self, name: impl Into<String>, method_hash: TypeHash) {
    self.methods
        .entry(name.into())
        .or_default()
        .push(method_hash);  // Adds even if derived class already has this method
}
```

## Solution

When copying methods from base class, check if the derived class already has a method with the **exact same signature** (not just name). If so, skip the inheritance - the derived class's method is the override and the base method should not be discoverable.

**Important:** Only skip when signatures match exactly. Name-only matching would break valid overloads:
```angelscript
class Base {
    void foo(int x) { }
}
class Derived : Base {
    void foo(string s) { }  // This is an OVERLOAD, not an override - both should exist
}
```

### Implementation

In `complete_class()`, check signature compatibility before adding inherited methods:

```rust
fn complete_class(&mut self, class_hash: TypeHash, output: &mut CompletionOutput) -> Result<bool, CompilationError> {
    // ... collect inherited methods ...

    // Get derived class's own methods (before inheritance)
    let own_methods: Vec<TypeHash> = class.methods.values().flatten().copied().collect();

    for (name, base_method_hash) in inherited.methods {
        // Skip if derived class has an override with EXACT same signature
        if self.has_matching_override(&own_methods, base_method_hash) {
            continue;  // Don't inherit - derived has override
        }
        class.add_method(name, base_method_hash);
        output.methods_inherited += 1;
    }
}

fn has_matching_override(&self, own_methods: &[TypeHash], base_method: TypeHash) -> bool {
    let base_func = self.get_function(base_method)?;

    own_methods.iter().any(|&own_hash| {
        let own_func = self.get_function(own_hash)?;
        signatures_match(&own_func.def, &base_func.def)
    })
}

fn signatures_match(a: &FunctionDef, b: &FunctionDef) -> bool {
    // Same name
    if a.name != b.name {
        return false;
    }
    // Same parameter count
    if a.params.len() != b.params.len() {
        return false;
    }
    // Same parameter types (in order)
    a.params.iter().zip(&b.params).all(|(pa, pb)| {
        pa.data_type.type_hash == pb.data_type.type_hash
    })
    // Note: return type covariance could be checked but not required for override detection
}
```

## Files to Modify

- `crates/angelscript-compiler/src/passes/completion.rs` - Filter inherited methods

## Test Case

```angelscript
class Animal {
    void speak() { print("Animal"); }
}

class Dog : Animal {
    void speak() { print("Woof"); }  // Override
}

void test() {
    Dog dog;
    dog.speak();  // Should call Dog::speak, not ambiguous
}
```

## Acceptance Criteria

- [ ] `cargo test --test unit test_inheritance` passes
- [ ] Override methods in derived class take precedence over base
- [ ] Non-overridden methods are still inherited correctly
- [ ] No regression in other inheritance tests
