# Phase 4b: Namespace Tree Design

> Part of the QualifiedName-Based Registry Architecture task

## Context

We are implementing a QualifiedName-based registry architecture for an AngelScript compiler. Currently in Phase 4 of 7.

**Current State:**
- `QualifiedName` has `simple_name: String` and `namespace: Vec<String>`
- Entry types (ClassEntry, InterfaceEntry, etc.) store redundant `name`, `namespace`, `qualified_name` fields alongside `qname`
- SymbolRegistry uses `FxHashMap<QualifiedName, TypeEntry>` for flat storage
- Type resolution requires string concatenation and multiple hash lookups

**Problem:**
When resolving names with `using` directives:
```angelscript
using Some::Other;
Namespace myClass;  // Should resolve to Some::Other::Namespace::myClass
```

Current approach:
1. Build full qualified strings for each candidate
2. Hash lookup for each possibility
3. No structure to traverse - just flat lookups

## Design Requirements

### 1. Namespace Tree Structure

Design a tree structure where:
- Each node represents a namespace
- Types/functions live in their namespace node (not flat storage)
- `using` directives become references between nodes
- Resolution is tree traversal, not string matching

```
(root)
├── Some
│   └── Other
│       └── Namespace
│           └── myClass → TypeEntry
└── Game
    └── Entities
        └── Player → TypeEntry
```

### 2. Remove Redundant Fields from Entry Types

**CRITICAL**: As part of this design, remove ALL redundant namespace/name storage from entry types.

Current redundant fields to REMOVE from `ClassEntry`, `InterfaceEntry`, `EnumEntry`, `FuncdefEntry`:
- `name: String` - redundant, use tree position or accessor
- `namespace: Vec<String>` - redundant, implicit from tree position
- `qualified_name: String` - redundant, can be computed from tree path

Entry types should only store:
- `type_hash: TypeHash`
- `source: TypeSource`
- Type-specific data (methods, fields, etc.)

The simple name can either:
- Be stored once in the tree node key
- Be accessed via a method that queries the tree

### 3. Update TypeEntry and Accessors

`TypeEntry::name()`, `TypeEntry::namespace()`, `TypeEntry::qualified_name()` currently return references to stored fields. These must change to:
- Return computed values (owned `String`)
- Or take the tree as a parameter to compute
- Or store a back-reference to the tree node

### 4. Type Resolution

Design the resolution algorithm:
```rust
fn resolve_type(&self, name: &str, from_node: &NamespaceNode) -> Option<&TypeEntry> {
    // 1. Check current node's types
    // 2. Walk up to parent nodes
    // 3. Check using directive references
    // 4. Check global namespace
}
```

### 4. Integration with Phases 5/6/7

The tree must support:
- **Phase 5 (Registration)**: Build tree incrementally while walking AST
- **Phase 6 (Completion)**: Resolve all `UnresolvedType` via tree traversal
- **Phase 7 (Compilation)**: Same resolution API for expression types

### 5. Hash Index for Bytecode

Still need `TypeHash` lookups for VM dispatch:
- Build reverse index after tree is complete
- Or store hash→node mappings

## Key Questions to Answer

1. **Node Structure**: What exactly does `NamespaceNode` contain?
   - Children map, types map, functions map?
   - Using references - how to handle cycles?
   - Parent pointer?

2. **Ownership**: Who owns the entries?
   - Tree nodes own entries directly?
   - Separate storage with tree holding references?

3. **Mutability**: How to handle registration vs lookup?
   - Build phase (mutable) vs resolve phase (immutable)?
   - Interior mutability?

4. **Template Instances**: Where do `array<int>` live?
   - Global namespace with full name?
   - Inside template definition's node?

5. **QualifiedName Fate**: Keep or remove?
   - Useful for debugging/display?
   - Keep as computed property only?
   - Remove entirely and compute path from tree?

6. **Entry Type Field Removal**: How to handle callers?
   - Many places access `entry.name`, `entry.qualified_name` directly
   - Need migration strategy for all call sites
   - Some callers need the name for error messages - how to provide it?

7. **FunctionDef**: Also has `name`, `namespace`, `qualified_name` - same treatment?

## Files to Reference

- `crates/angelscript-core/src/qualified_name.rs` - Current QualifiedName
- `crates/angelscript-registry/src/registry.rs` - Current SymbolRegistry
- `crates/angelscript-core/src/entries/*.rs` - Entry types (ClassEntry, InterfaceEntry, EnumEntry, FuncdefEntry)
- `crates/angelscript-core/src/entries/type_entry.rs` - TypeEntry enum with `name()`, `qualified_name()`, `namespace()` accessors
- `crates/angelscript-core/src/function_def.rs` - FunctionDef (also has name/namespace fields)
- `tasks/qualified_name_registry/*.md` - Phase designs (especially 05, 06, 07)

## Current Field Inventory (to be removed/changed)

### ClassEntry
- `qname: QualifiedName` - REMOVE or make computed
- `name: String` - REMOVE
- `namespace: Vec<String>` - REMOVE
- `qualified_name: String` - REMOVE

### InterfaceEntry
- `qname: QualifiedName` - REMOVE or make computed
- `name: String` - REMOVE
- `namespace: Vec<String>` - REMOVE
- `qualified_name: String` - REMOVE

### EnumEntry
- `qname: QualifiedName` - REMOVE or make computed
- `name: String` - REMOVE
- `namespace: Vec<String>` - REMOVE
- `qualified_name: String` - REMOVE

### FuncdefEntry
- `qname: QualifiedName` - REMOVE or make computed
- (name, namespace, qualified_name already removed in current branch)

### FunctionDef
- `name: String` - REMOVE or make computed
- `namespace: Vec<String>` - REMOVE
- `cached_qname: OnceCell<QualifiedName>` - REMOVE

### GlobalPropertyEntry
- `name: String` - REMOVE or make computed
- `namespace: Vec<String>` - REMOVE
- `qualified_name: String` - REMOVE

## Expected Output

1. Design document for namespace tree structure
2. Updated phase designs (5/6/7) showing tree integration
3. Migration plan from current flat storage
4. Plan for removing all redundant name/namespace/qualified_name fields

## Constraints

- Must support AngelScript's `using namespace` directive
- Must support nested namespaces (`namespace A::B::C { }`)
- Must handle forward references (type used before declared)
- Must support FFI types (registered from Rust, not script)
- Performance: Resolution should be O(depth) not O(total_types)
- All entry types must lose redundant name storage
