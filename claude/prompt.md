# Current Task: Implement Semantic Analysis - Phase 1 (Foundation & Symbol Collection)

**Status:** Ready to start
**Date:** 2025-11-24
**Phase:** Semantic Analysis - Pass 1 of 3

---

## Context

The parser is 100% complete (493 tests passing, ~1ms for 5000 lines). We're now implementing semantic analysis in 3 passes following Crafting Interpreters principles:

1. **Pass 1: Resolution & Symbol Collection** ← YOU ARE HERE
2. Pass 2: Type Resolution
3. Pass 3: Type Checking & Validation

**Reference:** See `/claude/semantic_analysis_plan.md` for complete architecture and design.

---

## Objective

Implement the foundation for semantic analysis and the first pass (symbol collection and name resolution).

**Goals:**
- Build infrastructure (error types, scope management, symbol tables)
- Collect all declarations from the AST
- Resolve variable/function references to their declarations
- Detect duplicate declarations and undefined names
- Handle forward references correctly

---

## What to Build (5 files)

### 1. Error Types (`src/semantic/error.rs`)

Create semantic error types following the same pattern as `ParseError`:

```rust
pub struct SemanticError {
    pub kind: SemanticErrorKind,
    pub span: Span,
    pub message: String,
}

pub enum SemanticErrorKind {
    // Symbol resolution errors
    UndefinedVariable,
    UndefinedFunction,
    UndefinedType,
    DuplicateDeclaration,
    UseBeforeDefinition,

    // Context errors
    ReturnOutsideFunction,
    BreakOutsideLoop,
    ContinueOutsideLoop,

    // Placeholder for future passes
    TypeMismatch,
    // ... more kinds added in later phases
}
```

**Requirements:**
- Include `display_with_source()` like ParseError
- Clear, helpful error messages
- Source span tracking

### 2. Scope Management (`src/semantic/scope.rs`)

Implement stack-based scope tracking (Crafting Interpreters style):

```rust
pub struct ScopeStack {
    scopes: Vec<ScopeId>,
    scope_data: HashMap<ScopeId, ScopeInfo>,
}

pub struct ScopeInfo {
    pub kind: ScopeKind,
    pub parent: Option<ScopeId>,
    pub symbols: HashMap<&'src str, SymbolId>,
}

pub enum ScopeKind {
    Global,
    Namespace(String),
    Class(String),
    Function(String),
    Block,
}
```

**Key operations:**
- `push_scope(kind)` - Enter new scope
- `pop_scope()` - Exit current scope
- `lookup(name)` - Search current scope and parents
- `declare(name, symbol)` - Add symbol to current scope

### 3. Symbol Table (`src/semantic/symbol_table.rs`)

Define symbol representation:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(u32);

pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub declared_type: Option<TypeExpr>,  // Unresolved yet
    pub span: Span,
    pub is_defined: bool,   // Two-phase: declared vs defined
}

pub enum SymbolKind {
    Variable,
    Parameter,
    Function,
    Class,
    Interface,
    Enum,
    Namespace,
    Field,
}
```

**Storage:**
```rust
pub struct SymbolTable {
    symbols: Vec<Symbol>,  // SymbolId is index
    // Fast access structures as needed
}
```

### 4. Built-in Allowlist (Don't Error on Known Built-ins)

**Problem:** Test scripts use application-registered types (`string`, `array<T>`) and functions (`print`, `sqrt`, etc.) that don't exist yet.

**Solution:** Simple allowlist - don't report "undefined" errors for known built-in names. This is just to prevent errors during testing.

Add to resolver:

```rust
/// Known built-in types/functions that will be registered by application
/// Just used to suppress "undefined" errors during testing
const KNOWN_BUILTINS: &[&str] = &[
    // Types
    "string",
    "array",
    "dictionary",

    // Functions
    "print",
    "sqrt",
    "abs",
    "pow",
    "min",
    "max",
    "sin",
    "cos",
    "tan",
    "floor",
    "ceil",
    "round",
];

impl Resolver {
    fn is_known_builtin(&self, name: &str) -> bool {
        KNOWN_BUILTINS.contains(&name)
    }

    fn lookup(&self, name: &str) -> Option<SymbolId> {
        // Try to find in scope chain
        if let Some(symbol_id) = self.scope_stack.lookup(name) {
            return Some(symbol_id);
        }

        // If not found but is known builtin, don't error
        // Return None but don't add to errors
        if self.is_known_builtin(name) {
            return None; // Treated as "found" - no error
        }

        // Truly undefined
        None
    }
}
```

**Why this approach:**
- Minimal code - just a simple list
- Test scripts won't error on built-ins
- Easy to extend as you find more
- No special symbol types needed
- No changes to `Symbol` or `SymbolKind`
- Will be removed when Engine registration API is implemented

**Usage:** When resolver encounters an undefined name, check if it's in the allowlist before reporting error.

**Note:** As you test with real scripts from `test_scripts/`, add any new built-in names to the `KNOWN_BUILTINS` list.

### 5. Resolver - Pass 1 (`src/semantic/resolver.rs`)

Main implementation following jlox Chapter 11 pattern:

```rust
pub struct Resolver<'src, 'ast> {
    // Scope tracking
    scope_stack: ScopeStack,

    // Results
    symbol_table: SymbolTable,
    resolutions: HashMap<NodeId, SymbolId>,  // AST node → symbol

    // Context tracking
    current_function: Option<FunctionKind>,
    current_class: Option<ClassKind>,
    in_loop: bool,

    // Errors
    errors: Vec<SemanticError>,
}

pub struct ResolutionData {
    pub symbol_table: SymbolTable,
    pub resolutions: HashMap<NodeId, SymbolId>,
    pub scope_tree: ScopeTree,
    pub errors: Vec<SemanticError>,
}

impl<'src, 'ast> Resolver<'src, 'ast> {
    pub fn resolve(ast: &Script<'src, 'ast>) -> ResolutionData {
        // Single O(n) traversal of AST
        // Collect all declarations
        // Resolve all name references
        // Check for errors
    }

    // Visitor methods for each AST node type
    fn visit_declaration(&mut self, decl: &Declaration) { }
    fn visit_statement(&mut self, stmt: &Statement) { }
    fn visit_expression(&mut self, expr: &Expression) { }

    // Symbol management (declare/define pattern)
    fn declare(&mut self, name: &str, kind: SymbolKind) { }
    fn define(&mut self, name: &str) { }
    fn lookup(&self, name: &str) -> Option<SymbolId> { }
}
```

**Algorithm (from plan):**
1. Walk AST with visitor pattern
2. For each block/function/class: push new scope
3. For each declaration: declare (but not define yet)
4. For initializers/bodies: visit and resolve references
5. After initializer: define the symbol (now usable)
6. For each variable use: resolve to declaration, record in `resolutions`
7. Pop scope when exiting block
8. Single O(n) traversal

**Semantic checks in this pass:**
- Duplicate declaration in same scope
- Undefined variable/function reference
- Use before definition (variable used in own initializer)
- Return outside function
- Break/continue outside loop

### 6. Module Coordinator (`src/semantic/mod.rs`)

```rust
pub mod error;
pub mod scope;
pub mod symbol_table;
pub mod resolver;

pub use error::{SemanticError, SemanticErrorKind};
pub use resolver::{resolve, ResolutionData};
pub use symbol_table::{Symbol, SymbolId, SymbolKind};
```

---

## Implementation Steps

### Step 1: Create Module Structure
```bash
mkdir -p src/semantic
touch src/semantic/mod.rs
touch src/semantic/error.rs
touch src/semantic/scope.rs
touch src/semantic/symbol_table.rs
touch src/semantic/resolver.rs
```

### Step 2: Implement Error Types
- Define `SemanticError` and `SemanticErrorKind`
- Implement `Display` and error formatting
- Add `display_with_source()` method
- Write basic tests

### Step 3: Implement Scope Management
- Define `ScopeStack`, `ScopeInfo`, `ScopeKind`
- Implement push/pop operations
- Implement lookup with scope chain walking
- Write tests for nested scopes and shadowing

### Step 4: Implement Symbol Table
- Define `Symbol`, `SymbolId`, `SymbolKind`
- Implement symbol storage
- Write tests for symbol operations

### Step 5: Implement Resolver (Core Logic)
- Define `Resolver` struct
- Add `KNOWN_BUILTINS` const array with allowlist
- Implement `is_known_builtin()` helper
- Modify `lookup()` to not error on known built-ins
- Implement visitor pattern for all AST nodes
- Implement declare/define logic
- Implement name resolution
- Handle all declaration types (function, class, variable, etc.)
- Track context (current function, in loop, etc.)
- Write comprehensive tests

### Step 6: Integration
- Update `src/lib.rs` to expose semantic module
- Add integration tests in `tests/semantic_tests.rs`
- Test with real AngelScript code samples from `test_scripts/`

---

## Test Coverage Requirements

Write tests for:

### Basic Resolution
- [x] Simple variable declaration and use
- [x] Function declaration and call
- [x] Forward function references work
- [x] Parameter scoping

### Scoping
- [x] Block scope shadowing
- [x] Function scope
- [x] Class member scope
- [x] Global scope

### Error Detection
- [x] Undefined variable error
- [x] Undefined function error
- [x] Duplicate declaration in same scope
- [x] Use before definition (self-reference)
- [x] Return outside function
- [x] Break/continue outside loop

### Complex Cases
- [x] Nested scopes (blocks within blocks)
- [x] Class members and methods
- [x] Multiple functions with same local names
- [x] Namespace resolution (if time permits)

**Target: 30-40 tests covering all scenarios**

---

## Performance Constraints

**Critical:** Pass 1 must complete in **< 1.0 ms for 5000 lines**

**Design for performance:**
- Use `FxHashMap` from `rustc_hash` (faster than std HashMap)
- Pre-allocate collections: `Vec::with_capacity(ast.declarations.len() * 4)`
- Use string slices `&'src str` not owned `String` where possible
- Mark hot functions with `#[inline]`
- Avoid allocations in inner loops

**Add benchmarks** in `benches/semantic_benchmarks.rs`:
```rust
group.bench_function("pass1_resolution_5000_lines", |b| {
    let (ast, _) = parse_lenient(stress_test);
    b.iter(|| resolver::resolve(&ast));
});
```

---

## Acceptance Criteria

Phase 1 is complete when:

- [ ] All 5 files created and compiling (error, scope, symbol_table, resolver, mod)
- [ ] Error types follow ParseError pattern
- [ ] Scope stack implements push/pop/lookup correctly
- [ ] Symbol table stores and retrieves symbols
- [ ] Built-in allowlist prevents errors for known names (string, array, print, sqrt, etc.)
- [ ] Resolver walks entire AST
- [ ] All declarations collected into symbol table
- [ ] All variable references resolved to declarations
- [ ] Duplicate declarations detected
- [ ] Undefined names reported (except allowlisted built-ins)
- [ ] Use-before-definition caught
- [ ] 30-40 tests passing
- [ ] Integration tests with real test scripts pass (functions.as, game_logic.as, etc.)
- [ ] Benchmarks show < 1ms for 5000 lines
- [ ] No compiler warnings
- [ ] All clippy lints passing
- [ ] Documented with rustdoc comments

---

## Example Usage (Target API)

```rust
use angelscript::{parse_lenient, semantic::resolve};

let source = r#"
    void foo() {
        bar();  // Forward reference - OK
    }

    void bar() { }

    void test() {
        int x = x;  // Use before definition - ERROR
        int x = 5;  // Duplicate - ERROR
        y = 10;     // Undefined - ERROR
    }
"#;

let (ast, parse_errors) = parse_lenient(source);
assert!(parse_errors.is_empty());

let resolution = resolve(&ast);
assert_eq!(resolution.errors.len(), 3);

// Check specific errors
assert!(resolution.errors.iter().any(|e|
    matches!(e.kind, SemanticErrorKind::UseBeforeDefinition)
));
assert!(resolution.errors.iter().any(|e|
    matches!(e.kind, SemanticErrorKind::DuplicateDeclaration)
));
assert!(resolution.errors.iter().any(|e|
    matches!(e.kind, SemanticErrorKind::UndefinedVariable)
));
```

---

## Reference Materials

- **Plan:** `/claude/semantic_analysis_plan.md` (sections: Pass 1, Performance)
- **Inspiration:** Crafting Interpreters Chapter 11 (Resolving and Binding)
- **Parser patterns:** `src/ast/parser.rs`, `src/ast/error.rs`
- **AST types:** `src/ast/node.rs`, `src/ast/decl.rs`, `src/ast/stmt.rs`, `src/ast/expr.rs`
- **Architecture:** `/docs/architecture.md`

---

## Key Design Principles

1. **Follow Crafting Interpreters** - Use jlox's resolver as a guide
2. **Single O(n) pass** - Visit each AST node exactly once
3. **Declare/define separation** - Prevents self-reference bugs
4. **Stack-based scopes** - Push/pop during traversal
5. **Side tables** - Don't modify AST, store results separately
6. **Performance first** - Pre-allocate, use efficient data structures
7. **Clear errors** - Helpful messages with source locations

---

## Notes

- **Don't implement type checking yet** - That's Pass 3
- **Don't resolve type names yet** - That's Pass 2
- **Focus on name resolution only** - Which declaration does this name refer to?
- **Defer all type work** - Just record that `int x` exists, don't resolve what `int` means yet

---

## Next Steps After Phase 1

Once Pass 1 is complete and tested:
1. Update `/claude/prompt.md` with Phase 2 prompt
2. Log design decisions in `/claude/decisions.md`
3. Begin Phase 2: Type Resolution

---

**Ready to implement! Start with error types, then scope management, then resolver.**
