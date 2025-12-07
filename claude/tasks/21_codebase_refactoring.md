# Task 21: Codebase Refactoring - AST and Semantic Modules

## Problem Statement

The codebase has grown complex and hard to reason about, particularly in the semantic and AST modules:

| Module | Lines | Key Issues |
|--------|-------|------------|
| **Semantic** | 37,727 | `function_processor/mod.rs` (12,335 lines, 472 functions), `registry.rs` (5,072 lines god object) |
| **AST** | 15,878 | `visitor.rs` (1,805 lines unused), `decl_parser.rs` (2,842 lines) |

### Root Causes

1. **Code Duplication**: `build_qualified_name()` duplicated 3x, `visit_namespace()` duplicated 3x
2. **God Objects**: `registry.rs` (170 functions), `function_processor/mod.rs` (472 functions)
3. **Dead Code**: `visitor.rs` is 1,805 lines with zero production usage
4. **Inconsistent Naming**: `Registrar` vs `TypeCompiler` vs `FunctionCompiler`
5. **Poor Testability**: Components tightly coupled, require full compiler setup to test

---

## Chosen Solution: Multi-Phase Refactoring

A 9-phase incremental refactoring that:
1. Extracts shared infrastructure for semantic passes
2. Splits large files into focused components
3. Creates workspace with separate crates
4. Establishes consistent naming conventions
5. Improves Rust idioms

**Guiding Principles:**
- Green tests always - Every commit passes `cargo test`
- One thing at a time - Each phase is focused and independent
- Tag before phases - `git tag before-phase-N` for easy rollback
- Facade pattern - Maintain API compatibility during transitions

---

## Phase 1: Semantic Pass Infrastructure

**Goal**: Eliminate code duplication and establish shared infrastructure for semantic passes.

**Status**: Pending

### 1.1 Create `NameResolutionContext`

All 3 passes duplicate the same state. Extract to shared struct:

```rust
// src/common/name_resolution.rs
pub struct NameResolutionContext {
    pub namespace_path: Vec<String>,
    pub imported_namespaces: Vec<String>,
}

impl NameResolutionContext {
    pub fn new() -> Self { ... }
    pub fn qualified_name(&self, name: &str) -> String { ... }
    pub fn enter_namespace(&mut self, path: &[&str]) { ... }
    pub fn exit_namespace(&mut self, count: usize) { ... }
    pub fn push_import(&mut self, ns: String) { ... }
    pub fn with_import_scope<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R { ... }
}
```

### 1.2 Create `SemanticPass` Trait

```rust
// src/semantic/pass.rs
pub trait SemanticPass<'ast> {
    fn context(&self) -> &NameResolutionContext;
    fn context_mut(&mut self) -> &mut NameResolutionContext;
    fn process_item(&mut self, item: &'ast Item<'ast>);

    // Default implementations for visit_script, visit_item, visit_namespace, visit_using_namespace
}
```

### 1.3 Migrate Passes

Update all 3 passes to use shared infrastructure:
- `src/semantic/passes/registration.rs`
- `src/semantic/passes/type_compilation.rs`
- `src/semantic/passes/function_processor/mod.rs`

**Files to create**:
- `src/common/mod.rs`
- `src/common/name_resolution.rs`
- `src/semantic/pass.rs`

**Estimated effort**: 3-4 hours

---

## Phase 2: Split function_processor into Testable Components

**Goal**: Break 12,335-line monolith into focused, independently testable components.

**Status**: Pending

### 2.1 Extract `ExpressionTypeChecker`

```rust
// src/semantic/passes/function_processor/expression_checker.rs
pub struct ExpressionTypeChecker<'a, 'ast> {
    registry: &'a Registry<'ast>,
    local_scope: &'a LocalScope,
    context: &'a NameResolutionContext,
    current_class: Option<TypeId>,
}

impl<'a, 'ast> ExpressionTypeChecker<'a, 'ast> {
    pub fn check(&mut self, expr: &'ast Expr<'ast>) -> Result<ExprContext, TypeError>;
    fn check_function_call(&mut self, call: &CallExpr) -> Result<ExprContext, TypeError>;
    fn check_method_call(&mut self, call: &CallExpr, receiver: ExprContext) -> Result<ExprContext, TypeError>;
    fn check_constructor_call(&mut self, call: &CallExpr) -> Result<ExprContext, TypeError>;
}
```

### 2.2 Extract `StatementCompiler`

```rust
pub struct StatementCompiler<'a, 'ast> {
    expr_checker: ExpressionTypeChecker<'a, 'ast>,
    bytecode: &'a mut BytecodeEmitter,
    local_scope: &'a mut LocalScope,
}
```

### 2.3 Extract `OverloadResolver`

```rust
pub struct OverloadResolver<'a, 'ast> {
    registry: &'a Registry<'ast>,
}

impl<'a, 'ast> OverloadResolver<'a, 'ast> {
    pub fn resolve(&self, candidates: &[FunctionId], arg_types: &[DataType]) -> Result<FunctionId, OverloadError>;
}
```

### 2.4 Target File Structure

```
src/semantic/passes/function_processor/
├── mod.rs                  (~800 lines) - Orchestrator
├── expression_checker.rs   (~2,000 lines)
├── statement_compiler.rs   (~800 lines)
├── overload_resolver.rs    (~300 lines)
├── call_checker.rs         (~600 lines)
├── operator_checker.rs     (~400 lines)
├── member_checker.rs       (~300 lines)
├── lambda_compiler.rs      (~400 lines)
├── bytecode_emitter.rs     (existing)
└── type_helpers.rs         (existing)
```

**Estimated effort**: 6-8 hours

---

## Phase 3: Split registry.rs

**Goal**: Decompose 5,072-line god object using facade pattern.

**Status**: Pending

**Target structure**:
```
src/semantic/types/
├── registry.rs         (~500 lines) - Facade
├── type_storage.rs     (~1,500 lines)
├── function_storage.rs (~1,200 lines)
├── template_cache.rs   (~800 lines)
├── ffi_importer.rs     (~600 lines)
└── (existing files)
```

**Estimated effort**: 3-4 hours

---

## Phase 4: Split decl_parser.rs

**Goal**: Reduce 2,842-line parser file.

**Status**: Pending

**Target structure**:
```
src/ast/
├── decl_parser.rs      (~400 lines) - Entry points
├── class_parser.rs     (~800 lines)
├── function_parser.rs  (~600 lines)
├── enum_parser.rs      (~300 lines)
└── namespace_parser.rs (~400 lines)
```

**Estimated effort**: 2-3 hours

---

## Phase 5: Create Workspace Structure

**Goal**: Extract into separate crates for faster compilation and clear boundaries.

**Status**: Pending

**Workspace structure**:
```
angelscript-rust/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── angelscript-core/         # Span, TypeId, DataType, primitives
│   ├── angelscript-parser/       # Lexer + AST + parser
│   ├── angelscript-compiler/     # Semantic analysis + codegen
│   ├── angelscript-ffi/          # FFI registration
│   └── angelscript-modules/      # Built-in modules
├── src/                          # Main angelscript crate
└── tests/
```

**Dependency graph** (no cycles):
```
              core
                ↑
        ┌───────┴───────┐
      parser         compiler
        ↑               ↑
        └───────────────┘
                ↑
              ffi ←── modules
                ↑
            angelscript
```

**Estimated effort**: 1-2 days

---

## Phase 6: Delete Unused Visitor Pattern

**Goal**: Remove 1,805 lines of dead code.

**Status**: Pending

### Analysis

The Visitor trait in `src/ast/visitor.rs` is completely unused in production:
- 1,805 lines, 45+ methods
- All `impl Visitor` are tests within the same file (self-referential)
- Semantic passes don't use it - they need return values and contextual parameters

**Why semantic passes CAN'T use Visitor:**

| Pass | Blocker |
|------|---------|
| Registration | Returns `TypeId`/`FunctionId`, needs `object_type` parameter |
| TypeCompilation | Returns `DataType`, builds span→type mapping |
| FunctionCompiler | Returns `ExprContext`, emits bytecode during traversal |

### Action

1. Delete `src/ast/visitor.rs` entirely
2. Update README.md to remove Visitor example
3. Update `src/ast/mod.rs` to remove visitor module export

**Estimated effort**: 30 minutes

---

## Phase 7: Naming Consistency

**Goal**: Establish consistent naming conventions across the codebase.

**Status**: Pending

### 7.1 Standardize Pass/Compiler Naming

```rust
// Before → After
Registrar → RegistrationPass
TypeCompiler → TypeResolutionPass
FunctionCompiler → FunctionCompilationPass
```

### 7.2 Standardize Output/Result Naming

```rust
RegistrationData → RegistrationOutput
TypeCompilationData → TypeResolutionOutput
CompilationResult → CompilationOutput
```

### 7.3 Standardize Method Naming Conventions

| Pattern | Usage | Example |
|---------|-------|---------|
| `get_*` | By ID, assumes exists | `get_type(type_id)` |
| `lookup_*` | By name, returns Option | `lookup_type(name)` |
| `find_*` | Complex matching/resolution | `find_compatible_function(...)` |
| `new()` | Simple construction | `TypeId::new()` |
| `build_*` | Complex/builder construction | `build_method_def(...)` |

### 7.4 Standardize Context/State Suffixes

| Suffix | Usage |
|--------|-------|
| `*Context` | Runtime execution environment |
| `*Scope` | Symbol/variable scoping |
| `*Info` | Compile-time metadata |
| `*State` | Mutable tracking state |

Rename: `ExprContext` → `ExprInfo`

**Estimated effort**: 2-3 hours

---

## Phase 8: Minor Cleanup (Lexer, Codegen, AST)

**Goal**: Address smaller quality issues in other modules.

**Status**: Pending

### 8.1 Lexer: Extract Keywords Module

`token.rs` (1,079 LOC) mixes TokenKind enum + keyword lookup. Extract `keywords.rs`.

### 8.2 Codegen: Extract Break Context Manager

Extract `BreakContextManager` from `BytecodeEmitter`.

### 8.3 AST: Consolidate Parser Organization

When moving to workspace, organize parsers by concern:
```
parser/
├── mod.rs
├── expressions.rs
├── statements.rs
├── declarations/
│   ├── mod.rs
│   ├── class.rs
│   ├── function.rs
│   └── namespace.rs
└── types.rs
```

**Estimated effort**: 2-3 hours (can be done during workspace migration)

---

## Phase 9: Rust Idiom Improvements

**Goal**: Make the code more idiomatic and reduce unnecessary allocations.

**Status**: Pending

### 9.1 Make `DataType` Copy

Currently cloned 175+ times. If all fields can be `Copy`, eliminates allocation overhead.

```rust
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct DataType { ... }
```

### 9.2 Add Display Traits

```rust
impl Display for DataType { ... }
impl Display for SemanticError { ... }
```

### 9.3 Replace String Comparisons with Enums

```rust
// Before
if name == "this" { ... }

// After
enum SpecialIdent { This, Super, ... }
```

### 9.4 Iterator Improvements

```rust
// Before
let mut name = String::new();
for seg in &namespace_path {
    if !name.is_empty() { name.push_str("::"); }
    name.push_str(seg);
}

// After
let name = namespace_path.join("::");
```

**Estimated effort**: 3-4 hours

---

## Implementation Order

| Order | Phase | Risk | Impact | Time |
|-------|-------|------|--------|------|
| 1 | Semantic pass infrastructure | Low | High | 3-4h |
| 2 | Split function_processor | Medium | Very High | 6-8h |
| 3 | Split registry | Medium | High | 3-4h |
| 4 | Split decl_parser | Low | Medium | 2-3h |
| 5 | Workspace structure | Low | High | 1-2 days |
| 6 | Delete unused Visitor | Very Low | Low | 30m |
| 7 | Naming consistency | Low | Medium | 2-3h |
| 8 | Minor cleanup | Very Low | Low | 2-3h |
| 9 | Rust idiom improvements | Low | Medium | 3-4h |

**Total estimated time**: 6-7 days of focused work

---

## Critical Files

**Highest priority** (split these first):
- `src/semantic/passes/function_processor/mod.rs` (12,335 lines)
- `src/semantic/types/registry.rs` (5,072 lines)

**Secondary priority**:
- `src/semantic/passes/type_compilation.rs` (3,397 lines)
- `src/semantic/const_eval.rs` (3,381 lines)
- `src/ast/decl_parser.rs` (2,842 lines)

**Dead code removal**:
- `src/ast/visitor.rs` (1,805 lines - DELETE entirely)

**New files to create**:
- `src/common/mod.rs`, `src/common/name_resolution.rs`
- `src/semantic/pass.rs`
- `src/semantic/passes/function_processor/expression_checker.rs`
- `src/semantic/passes/function_processor/statement_compiler.rs`
- `src/semantic/passes/function_processor/call_checker.rs`
- `src/semantic/passes/function_processor/operator_checker.rs`
- Multiple new files in `types/` for registry split
- `crates/*/` directories for workspace

---

## Success Criteria

- [ ] No file exceeds 2,000 lines
- [ ] No single struct has >50 methods in one file
- [ ] Zero duplicated utility functions (`build_qualified_name`, `visit_namespace`, etc.)
- [ ] All 2,350+ tests pass
- [ ] Workspace crates compile independently
- [ ] Incremental compilation improved by 3-4x
- [ ] `DataType` is `Copy` (or clones reduced by 80%+)
- [ ] User-facing types implement `Display`
- [ ] Components are independently testable (ExpressionTypeChecker, OverloadResolver, etc.)
- [ ] All semantic passes use shared `NameResolutionContext` and `SemanticPass` trait
- [ ] Consistent naming: all passes use `*Pass`, outputs use `*Output`, methods follow `get_`/`lookup_`/`find_` conventions

---

## Validation Checklist

### Per Phase
- [ ] `cargo test --lib` passes
- [ ] `cargo build --lib` succeeds
- [ ] `cargo clippy` has no new warnings
- [ ] Git tag created before starting phase
- [ ] Commit created after completing phase

### Final
- [ ] All benchmarks still pass
- [ ] Integration tests pass
- [ ] Documentation updated where needed
