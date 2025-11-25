# Architectural Decisions Log

This document records significant architectural decisions made during the development of the AngelScript Rust implementation.

---

## 2025-11-25: Simplified to 2-Pass Registry-Only Model

### Context

Initially planned a 3-pass semantic analysis architecture:
1. Pass 1: Resolution & Registration (collect symbols with SymbolTable)
2. Pass 2: Type Compilation (fill type details)
3. Pass 3: Function Compilation (type check + codegen)

After implementing Pass 1 with `Resolver` and `SymbolTable`, we realized the architecture had unnecessary complexity:
- SymbolTable stored both global AND local symbols
- Globals (types, functions) were duplicated between SymbolTable and planned Registry
- Local variables don't need global storage - they're compilation state

### Options Considered

**Option 1: Keep 3-pass with SymbolTable**
- Continue with planned architecture
- SymbolTable for all symbols (global + local)
- Registry for types
- Pros: Already partially implemented
- Cons: Duplication, complexity, doesn't match AngelScript C++

**Option 2: 2-pass Registry-only model** ✅ CHOSEN
- Pass 1: Registration (globals only in Registry)
- Pass 2: Compilation & Codegen (type compilation + function compilation)
  - Sub-phase 2a: Fill type details
  - Sub-phase 2b: Per-function compilation with LocalScope
- Pros: Simpler, matches AngelScript C++, clear separation
- Cons: Need to refactor existing Pass 1 code

**Option 3: Keep SymbolTable for Pass 1 output**
- SymbolTable captures Pass 1 results (for testing/inspection)
- Registry handles actual type system
- Pros: Preserves testability of Pass 1
- Cons: Still has duplication

### Decision

**Chosen: Option 2 - 2-pass Registry-only model**

### Rationale

1. **Matches AngelScript C++ proven architecture:**
   ```cpp
   ParseScripts();       // Parse
   CompileClasses();     // Pass 1: Register + fill type details
   CompileFunctions();   // Pass 2: Compile + codegen
   ```

2. **Clearer separation of concerns:**
   - **Registry = Global names** (types, functions, global variables)
   - **LocalScope = Local names** (per-function compilation state)
   - No overlap, no duplication

3. **Simpler implementation:**
   - One data structure for globals (Registry)
   - Temporary structure for locals (LocalScope)
   - No global tracking of local variables

4. **Performance benefits:**
   - Registry uses FxHashMap with qualified names
   - No need to track locals until compilation
   - Fixed TypeIds for primitives (no dynamic registration)

5. **Natural fit for Engine architecture:**
   - Registry IS the Engine's type system
   - Can be used directly at runtime
   - Not just a compiler intermediate structure

### Implementation Plan

**Phase 1: Implement new structures (Pass 2a focus)**
1. Create `Registry`, `DataType`, `TypeDef`
2. Create `TypeCompiler` (Pass 2a)
3. Keep existing `Resolver` and `SymbolTable` (don't break tests)

**Phase 2: Implement Registration (Pass 1)**
1. Create simplified `Registrar` (uses Registry directly)
2. Integrate with Pass 2a

**Phase 3: Implement Function Compilation (Pass 2b)**
1. Create `LocalScope` for per-function variable tracking
2. Create `FunctionCompiler` (type checking + codegen)

**Phase 4: Cleanup**
1. Remove `SymbolTable` (replaced by Registry + LocalScope)
2. Remove or simplify `Resolver` (replaced by Registrar)

### Consequences

**Positive:**
- ✅ Simpler architecture
- ✅ Faster implementation (less code to write)
- ✅ Better performance (no global local variable tracking)
- ✅ Matches proven AngelScript pattern
- ✅ Registry becomes the Engine type system

**Negative:**
- ⚠️ Need to refactor existing Pass 1 code
- ⚠️ Existing tests will need updates
- ⚠️ Documentation needs rewrite

**Neutral:**
- Pass count changes from 3 to 2 (with 2 sub-phases in Pass 2)
- Total work remains similar, but better organized

### References

- AngelScript C++ source: `as_builder.cpp`, `as_compiler.cpp`
- Original 3-pass plan: `/claude/semantic_analysis_plan.md` (archived)
- New 2-pass plan: `/claude/semantic_analysis_plan.md` (updated 2025-11-25)

---

## 2025-11-25: Pass 2a Implementation - Type Compilation Complete

### Context

With Pass 1 (Registration) complete, we needed to implement Pass 2a (Type Compilation) to fill in all type details in the Registry. This phase resolves all TypeExpr nodes from the AST into complete DataType instances and updates the Registry with complete type information.

### Implementation Details

**Files Created:**
- `src/semantic/type_compiler.rs` (~600 lines, 7 tests)

**Files Modified:**
- `src/semantic/registry.rs` (added update methods)
- `src/semantic/mod.rs` (added exports)

**Key Features Implemented:**
1. TypeExpr → DataType resolution
2. Class field type resolution
3. Class inheritance tracking
4. Interface method signature resolution
5. Function signature completion
6. Template instantiation (via Registry cache)
7. Comprehensive error handling

### Design Decisions

**1. Type Resolution Strategy**
- Chose to resolve types in a single-pass traversal
- Store resolved types in a `type_map` (Span → DataType) for later reference
- Handle scoped types (Namespace::Type) by checking current namespace first, then global
- **Rationale:** Keeps compilation fast and memory efficient

**2. Class Namespace Handling**
- Methods are registered with the class name as part of their namespace path
- Example: `class Player { void update() }` → method name is `Player::update`
- **Rationale:** Matches AngelScript C++ behavior and enables method lookup

**3. Inheritance Tracking**
- First item in class inheritance list is the base class (if it's a class type)
- Remaining items are interfaces
- Stored in separate maps: `inheritance` (Derived → Base) and `implements` (Class → [Interfaces])
- **Rationale:** Enables efficient hierarchy queries and validation

**4. Registry Update Pattern**
- Added `update_*` methods to Registry instead of direct mutation
- Methods: `update_class_details()`, `update_interface_details()`, etc.
- **Rationale:** Encapsulates Registry internal structure, cleaner API

**5. Error Handling**
- Errors are collected but compilation continues
- Failed type resolutions return None and skip that item
- **Rationale:** Report all errors in one pass, don't stop at first error

### Test Results

```
✅ 141 total semantic tests passing
✅ 7 new type_compiler tests
✅ 0 compiler warnings
✅ All clippy lints passing
```

**Test Coverage:**
- Primitive type resolution
- User-defined type resolution
- Type modifiers (const, @, const @)
- Class field resolution
- Inheritance handling

### Performance

- Single-pass O(n) traversal of AST
- Uses FxHashMap for fast lookups
- TypeId (u32) comparisons instead of strings
- Template instantiation cached in Registry

### Consequences

**Positive:**
- ✅ Type system now complete and usable
- ✅ Registry has full type information
- ✅ Clean separation from Pass 1
- ✅ Ready for Pass 2b (function body compilation)
- ✅ Comprehensive error reporting

**Negative:**
- ⚠️ Need more comprehensive tests for edge cases
- ⚠️ Circular inheritance detection not yet implemented (TODO)

**Next Steps:**
- Pass 2b: Function body compilation with LocalScope
- Add more test cases for templates and complex inheritance
- Performance benchmarking with large codebases

---

## Future Decisions

(To be added as we make them)
