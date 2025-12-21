# Task 47b5: Typedef Support

## Problem

Type aliases declared with `typedef` are not recognized.

**Errors:**
- `UnknownType { name: "EntityId" }`
- `UnknownType { name: "StringArray" }`

**Affected Test:** `test_types`

## Root Cause

The registration pass doesn't handle `typedef` declarations. When encountering:
```angelscript
typedef int EntityId;
typedef array<string> StringArray;
```

The compiler doesn't create entries that map `EntityId` -> `int` and `StringArray` -> `array<string>`.

## Context

AngelScript typedefs are simple type aliases - they create an alternative name for an existing type, but don't create a new distinct type.

## Solution

### Option A: Store typedefs as type aliases in registry

Add a new registry for type aliases:

```rust
// In SymbolRegistry or CompilationContext
type_aliases: HashMap<TypeHash, TypeHash>,  // alias_hash -> target_hash

fn resolve_type(&self, name: &str) -> Option<TypeHash> {
    // First check direct types
    if let Some(hash) = self.types.get(name) {
        return Some(*hash);
    }

    // Then check aliases
    let alias_hash = TypeHash::from_name(name);
    if let Some(&target) = self.type_aliases.get(&alias_hash) {
        return Some(target);
    }

    None
}
```

### Option B: Register typedefs during Pass 1

In registration pass, when encountering a typedef:

```rust
fn register_typedef(&mut self, typedef: &TypedefDecl) -> Result<()> {
    let alias_name = typedef.alias;  // "EntityId"
    let target_type = self.resolve_type(&typedef.target)?;  // int

    // Register the alias
    self.ctx.register_type_alias(alias_name, target_type.type_hash);
    Ok(())
}
```

## Files to Modify

- `crates/angelscript-compiler/src/passes/registration.rs` - Handle typedef declarations
- `crates/angelscript-registry/src/lib.rs` or `crates/angelscript-compiler/src/context.rs` - Store/lookup aliases
- `crates/angelscript-compiler/src/type_resolver.rs` - Resolve aliases during type resolution

## Parser Check

Verify that `typedef` declarations are parsed. Check:
- `crates/angelscript-parser/src/ast/` for TypedefDecl or similar
- If not parsed, this requires parser work first

## Test Case

```angelscript
typedef int EntityId;
typedef array<string> StringArray;

EntityId playerId = 42;
StringArray names;
names.insertLast("Alice");
```

## Acceptance Criteria

- [ ] `cargo test --test unit test_types` passes (typedef portion)
- [ ] Simple typedefs (`typedef int Foo`) work
- [ ] Template typedefs (`typedef array<int> IntArray`) work
- [ ] Typedef in namespaces works
- [ ] Typedef visibility respects scope
