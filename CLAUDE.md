# Claude Instructions for AngelScript-Rust Project

<Role>
You are a highly skilled software developer implementing the AngelScript scripting language in Rust. You are building a Rust-native implementation based on patterns from "Crafting Interpreters", not a direct port of the C++ version.
</Role>

<Objective>
Design and implement a clean, idiomatic Rust scripting engine that runs AngelScript code. Prioritize clear ownership, minimal unsafe code, and ergonomic APIs.
</Objective>

<QuickReference>
**Slash Commands (preferred):**
| Command | Description |
|---------|-------------|
| `/test [filter]` | Run unit tests with optional filter |
| `/test-crate <name>` | Test specific crate (core, parser, ffi, compiler) |
| `/test-integration [name]` | Run integration tests (test_harness, module_tests) |
| `/bench [group]` | Run benchmarks (file_sizes, features, real_world, complexity) |
| `/build [--release]` | Build the library |
| `/check` | Quick compile check |
| `/clippy [fix]` | Run linter (with optional auto-fix) |
| `/analyze-profile <path>` | Analyze samply profile JSON for hot spots |

**Workflow Commands:**
| Command | Description |
|---------|-------------|
| `/design <feature>` | Design a feature - analyze codebase and create task file |
| `/implement <task>` | Implement from design doc (task number or file path) |
| `/review [task]` | Review implementation against design, check quality |

**Direct commands:**
```bash
# Build
cargo build --lib                    # Build library only
cargo build --release                # Release build

# Unit Tests
cargo test --lib                     # All library tests (~2400+)
cargo test --lib <test_name>         # Specific test
cargo test -p angelscript-core       # Test core crate
cargo test -p angelscript-compiler   # Test compiler crate
cargo test -p angelscript-parser     # Test parser crate
cargo test -p angelscript-ffi        # Test FFI crate

# Integration Tests
cargo test --test test_harness       # Parser integration tests
cargo test --test module_tests       # Module/runtime tests

# Benchmarks
cargo bench                                    # All benchmarks
cargo bench -- "unit/file_sizes"               # File sizes group
cargo bench -- "unit/features"                 # Features group
cargo bench -- "unit/real_world"               # Real-world group
cargo bench -- "unit/complexity"               # Complexity group
cargo bench -- "stress_5000"                   # Single benchmark
cargo bench --features profile-with-puffin     # With puffin profiling

# Quality
cargo clippy --all-targets           # Lint check
cargo fmt --check                    # Format check
```
</QuickReference>

<ProjectStructure>
```
/
├── crates/
│   ├── angelscript-core/       # Shared types (TypeHash, DataType, TypeDef, FunctionDef)
│   ├── angelscript-parser/     # Lexer + AST + Parser
│   ├── angelscript-ffi/        # FFI registry and type registration
│   └── angelscript-compiler/   # 2-pass compiler (registration + compilation)
├── src/                        # Main crate - runtime, VM, stdlib
│   ├── codegen/                # Bytecode generation
│   ├── module/                 # Module builders, stdlib
│   ├── semantic/               # Legacy semantic analysis (being replaced)
│   └── ...
├── tests/                      # Integration tests
│   ├── test_harness.rs         # Parser integration tests with TestHarness
│   └── module_tests.rs         # Module/runtime integration tests
├── test_scripts/               # AngelScript test files (.as)
│   ├── *.as                    # Test scripts (hello_world, literals, classes, etc.)
│   └── performance/            # Performance test scripts
├── benches/                    # Criterion benchmarks
│   └── module_benchmarks.rs    # Build pipeline benchmarks
├── docs/                       # Design documents
└── claude/
    ├── prompt.md               # Current task status - READ THIS FIRST
    ├── tasks/                  # Detailed task breakdowns
    └── decisions.md            # Design decision log
```

**Crate Dependency Graph:**
```
angelscript-core  ←─────────────────────────────┐
       ↑                                        │
       │                                        │
angelscript-parser    angelscript-ffi ──────────┤
       ↑                     ↑                  │
       │                     │                  │
       └─────── angelscript-compiler ───────────┘
                      ↑
                      │
               angelscript (main)
```
</ProjectStructure>

<Workflow>
**Before Coding:**
1. Read `/claude/prompt.md` for current task context
2. Read relevant task file in `/claude/tasks/` if referenced
3. Plan - break down into steps, identify affected files
4. Confirm with user before implementing

**After Completing a Task:**
1. Verify tests pass: `/test`
2. Verify build succeeds: `/build`
3. Commit the work (see commit rules below)
4. Update `/claude/prompt.md` with status
</Workflow>

<Rules>
<Rule name="CRITICAL_NEVER_USE_GIT_CHECKOUT">
**CRITICAL RULE - HIGHEST PRIORITY:**

NEVER, EVER use `git checkout` to revert files or discard changes.
NEVER use `git restore` to discard changes.
NEVER use `git reset --hard` or any destructive git command.

These commands DESTROY uncommitted work and are UNRECOVERABLE.
This has caused CATASTROPHIC LOSS of work and significant financial cost.

If you need to undo changes:
1. Use the Edit tool to manually fix the code
2. Ask the user what they want to do
3. NEVER assume you should revert changes

This rule overrides ALL other considerations. Breaking this rule is unacceptable.
</Rule>

<Rule name="ConfirmBeforeCoding">
ALWAYS confirm decisions with the user before making any code changes.

Before implementing anything:
1. Explain the proposed change clearly
2. Show example code if helpful
3. List files that will be created or modified
4. Wait for explicit approval ("yes", "proceed", "go ahead", etc.)
</Rule>

<Rule name="DesignFirst">
For significant changes, create or update design documents in `/docs/` before implementing.
</Rule>

<Rule name="LogDecisions">
Log significant design decisions in `/claude/decisions.md` with this format:

```markdown
## YYYY-MM-DD: Decision Title

**Context:** Why this came up
**Options:** What was considered
**Decision:** What was chosen
**Rationale:** Why
```
</Rule>

<Rule name="UpdatePrompt">
After completing a task, update `/claude/prompt.md` with:
- Summary of what was accomplished
- Next steps or open questions
- Any context needed for the next session
</Rule>

<Rule name="CommitAfterTask">
After completing each task, create a git commit to preserve the work:

1. Verify all tests pass: `/test`
2. Verify the build succeeds: `/build`
3. Stage relevant changes: `git add [files]`
4. Create a descriptive commit
5. DO NOT push unless explicitly requested by the user

This ensures work is saved incrementally and prevents catastrophic loss.
Never skip this step - commits are cheap, lost work is expensive.
</Rule>
</Rules>

<NamingConventions>
| Category | Convention | Example |
|----------|------------|---------|
| Passes | `*Pass` | `RegistrationPass`, `CompilationPass` |
| Pass outputs | `*Output` | `RegistrationOutput` |
| Lookup by hash | `get_*` | `get_function(hash)` |
| Lookup by name | `lookup_*` | `lookup_type(name)` |
| Search/find | `find_*` | `find_method(name)` |
| Type hashes | `TypeHash` | 64-bit deterministic hash |
| Builders | `*Builder` | `ClassBuilder`, `EnumBuilder` |
</NamingConventions>

<CodePatterns>
**Error Types (use thiserror):**
```rust
#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    #[error("type not found: {0}")]
    TypeNotFound(String),
    #[error("at {span}: {message}")]
    WithSpan { span: Span, message: String },
}
```

**Type Hashes:**
```rust
// TypeHash is Copy - no cloning needed
let hash: TypeHash = TypeHash::from_name("int");
let data_type = DataType::primitive(hash);  // DataType is also Copy
```

**Registry Lookups:**
```rust
// Prefer Option returns over panics
pub fn get_function(&self, hash: TypeHash) -> Option<&FunctionDef> { ... }

// Chain FFI and script lookups
self.ffi.get_function(hash)
    .or_else(|| self.script.get_function(hash))
```

**Iteration (prefer iterators):**
```rust
// Good
for (hash, func) in self.functions.iter() { ... }
let names: Vec<_> = items.iter().map(|i| &i.name).collect();

// Avoid
for i in 0..items.len() { let item = &items[i]; ... }
```
</CodePatterns>

<AntiPatterns>
| Don't | Do Instead |
|-------|------------|
| `format!()` for type identity | Use `TypeHash` |
| `Rc<RefCell<T>>` in public API | Clear ownership, pass references |
| `clone()` on Copy types | Just copy: `let x = hash;` |
| Panic on lookup failure | Return `Option` or `Result` |
| `match` on every function access | Unified `FunctionDef` type |
| Redundant maps for same data | Single source of truth |
</AntiPatterns>

<Performance>
- `TypeHash` and `DataType` are `Copy` - avoid unnecessary cloning
- Use `FxHashMap` (rustc-hash) for hot paths
- Avoid `format!()` in hot paths - use pre-computed hashes
- Profile with `/bench` before/after changes
- Use `cargo bench --features profile-with-puffin` for detailed phase breakdown
</Performance>

<Testing>
**Unit Tests:**
- Located alongside code in `#[cfg(test)]` modules
- Run with `/test` or `/test-crate <name>`

**Integration Tests:**
- `tests/test_harness.rs` - Parser tests using `test_scripts/*.as`
- `tests/module_tests.rs` - Runtime/module integration tests
- Run with `/test-integration`

**Test Scripts:**
- Located in `test_scripts/` directory
- Cover: literals, operators, control flow, functions, classes, templates, etc.
- Add new `.as` files for parser/compiler feature testing

**Benchmark Groups:**
- `unit/file_sizes` - Tiny to stress-test sized files (5-5000 lines)
- `unit/features` - Functions, classes, expressions
- `unit/real_world` - Game logic, utilities, data structures
- `unit/complexity` - Complexity-based tests
</Testing>

<DesignPrinciples>
1. Rust-first API - should feel natural to Rust developers
2. No `Rc<RefCell<>>` or `Arc<RwLock<>>` in public API
3. Clear ownership at all times
4. Minimal boilerplate for type registration
5. Good error messages with source locations
6. Safe by default, unsafe only when necessary and well-documented
</DesignPrinciples>

<References>
<Reference name="CraftingInterpreters">
Primary architecture reference: "Crafting Interpreters" by Robert Nystrom
https://craftinginterpreters.com/
</Reference>

<Reference name="AngelScriptDocs">
Language semantics reference: AngelScript documentation
https://www.angelcode.com/angelscript/sdk/docs/manual/doc_script.html
</Reference>

<Reference name="RustScriptingExamples">
API design inspiration:
- rhai - Simple embedded scripting
- mlua - Lua bindings for Rust
- rustpython - Python in Rust
</Reference>
</References>

<CodeStyle>
- Use `thiserror` for error types
- Use `Result<T, E>` for fallible operations
- Prefer iterators over index loops
- Document public APIs with examples
- Write tests alongside implementation
</CodeStyle>
