# Task 26: Compiler Rewrite - 2-Pass Architecture

## Problem Summary

The TypeHash refactor (Task 22) caused 10-90% performance regression. Rather than surgically fix the existing complex 3-pass architecture, we'll create a clean 2-pass implementation in a new crate.

## Solution: New `crates/angelscript-compiler` with Clean 2-Pass Architecture

Combines:
- **Task 22 perf fix**: Proper type resolution, no format!() hashing
- **Task 21 Phase 5**: Workspace structure with separate crates
- **Task 21 Phase 2**: Split function_processor into testable components
- **Task 21 Phase 6**: Delete unused visitor.rs (1,805 lines dead code)
- **Task 21 Phase 7**: Consistent naming (`*Pass`, `*Output`, `get_`/`lookup_`/`find_`)
- **Task 21 Phase 9**: Rust idioms (DataType as Copy, Display traits)

---

## Session-Sized Tasks

Each task is designed to be completable in a single session without context overflow.

| # | Task | Description | Dependencies | Status |
|---|------|-------------|--------------|--------|
| 1 | Workspace Setup | Create workspace Cargo.toml, crate skeleton, lib.rs with re-exports | None | Complete |
| 2 | Types: TypeHash | Move TypeHash to compiler crate, add Display, make Copy | 1 | Complete |
| 3 | Types: DataType | Move DataType, make Copy, add Display, RefModifier | 2 | Complete |
| 4 | Types: TypeDef + FunctionDef | Create clean TypeDef and FunctionDef structs | 3 | Complete |
| 5 | Types: ExprInfo | Create ExprInfo (renamed from ExprContext) | 3 | Pending |
| 6 | ScriptRegistry | Implement clean registry (no redundant maps) | 4 | Pending |
| 7 | CompilationContext | Implement context with name resolution | 6 | Pending |
| 8 | Pass 1: RegistrationPass | Type + function registration with complete signatures | 7 | Pending |
| 9 | Pass 2: Orchestrator | CompilationPass mod.rs + BytecodeEmitter integration | 7 | Pending |
| 10 | Pass 2: OverloadResolver | Function overload resolution (standalone, testable) | 7 | Pending |
| 11 | Pass 2: ExpressionChecker | Expression type checking | 9, 10 | Pending |
| 12 | Pass 2: CallChecker | Function/method/constructor calls | 11 | Pending |
| 13 | Pass 2: OperatorChecker | Binary/unary operator overloads | 11 | Pending |
| 14 | Pass 2: MemberChecker + Lambda | Member access + lambda compilation | 11 | Pending |
| 15 | Pass 2: StatementCompiler | Statement compilation + bytecode | 11-14 | Pending |
| 16 | Integration | Wire up to main crate, feature flag, test against old | 15 | Pending |
| 17 | Cleanup | Delete old code, remove feature flag, final benchmarks | 16 | Pending |

---

## Architecture

### Current (in `src/semantic/passes/`):
```
Pass 1 (registration.rs):     Register types + functions (EMPTY params, format!() hash)
Pass 2a (type_compilation.rs): Walk AST again, fill signatures
Pass 2b (function_processor/): Walk AST again, type check + bytecode
```

### New (in `crates/angelscript-compiler/`):
```
Pass 1 (registration.rs):  Register types → Register functions with COMPLETE signatures
Pass 2 (compilation/):     Type check function bodies + generate bytecode
```

---

## Crate Structure

### Workspace Cargo.toml

```toml
# /Cargo.toml
[workspace]
members = [
    "crates/angelscript-compiler",
    ".",  # Main crate (angelscript)
]
```

### `crates/angelscript-compiler`

**Note:** We put shared types (TypeHash, DataType, TypeDef) in compiler for now.
The main crate's FFI code depends on compiler for these types.
We can extract `crates/angelscript-core` later when boundaries are clearer.

```
crates/angelscript-compiler/
├── Cargo.toml              # name = "angelscript-compiler"
└── src/
    ├── lib.rs               # Re-exports types for FFI to use
    ├── types/
    │   ├── mod.rs
    │   ├── type_hash.rs     # TypeHash, primitive_hashes (moved from src/types/)
    │   ├── data_type.rs     # DataType, RefModifier (moved, now Copy)
    │   ├── type_def.rs      # TypeDef
    │   ├── function_def.rs  # FunctionDef
    │   └── expr_info.rs     # ExprInfo (renamed from ExprContext)
    ├── context.rs           # CompilationContext (includes name resolution)
    ├── registry.rs          # Clean ScriptRegistry (no redundant maps)
    └── passes/
        ├── mod.rs
        ├── registration.rs      # Pass 1: types + complete function signatures
        └── compilation/         # Pass 2: split into testable components
            ├── mod.rs           # CompilationPass orchestrator (~500 lines)
            ├── expr_checker.rs  # ExpressionChecker (~1,500 lines)
            ├── stmt_compiler.rs # StatementCompiler (~600 lines)
            ├── overload.rs      # OverloadResolver (~300 lines)
            ├── call_checker.rs  # Function/method call checking (~500 lines)
            ├── op_checker.rs    # Operator overload checking (~400 lines)
            ├── member_checker.rs # Member access checking (~300 lines)
            └── lambda.rs        # Lambda compilation (~400 lines)
```

Main crate FFI uses:
```rust
// src/ffi/types.rs
use angelscript_compiler::{TypeHash, DataType, TypeDef};
```

---

## Key Components

### ScriptRegistry

**File:** `crates/angelscript-compiler/src/registry.rs`

No redundant maps, no update methods:

```rust
pub struct ScriptRegistry<'ast> {
    // Types - single map, TypeHash as key
    types: FxHashMap<TypeHash, TypeDef>,
    type_by_name: FxHashMap<String, TypeHash>,

    // Functions - single map, func_hash as key
    functions: FxHashMap<TypeHash, FunctionDef<'ast>>,
    func_by_name: FxHashMap<String, Vec<TypeHash>>,

    // Behaviors
    behaviors: FxHashMap<TypeHash, TypeBehaviors>,

    // NO types_by_hash (redundant)
    // NO update_* methods (functions registered complete)
    // NO signature_filled field (always complete)
}

impl<'ast> ScriptRegistry<'ast> {
    pub fn register_type(&mut self, typedef: TypeDef) -> TypeHash { ... }
    pub fn register_function(&mut self, func: FunctionDef<'ast>) -> TypeHash { ... }

    pub fn get_type(&self, hash: TypeHash) -> Option<&TypeDef> {
        self.types.get(&hash)  // Single lookup!
    }

    pub fn lookup_type(&self, name: &str) -> Option<TypeHash> {
        self.type_by_name.get(name).copied()
    }

    // No update_function_signature - not needed
    // No update_function_params - not needed
}
```

### CompilationContext (with Name Resolution)

**File:** `crates/angelscript-compiler/src/context.rs`

Unified context that holds registry + namespace tracking:

```rust
pub struct CompilationContext<'ast> {
    // Registries
    ffi: Arc<FfiRegistry>,
    script: ScriptRegistry<'ast>,

    // Namespace tracking (merged from NameResolutionContext)
    namespace_path: Vec<String>,
    imported_namespaces: Vec<String>,
}

impl<'ast> CompilationContext<'ast> {
    pub fn new(ffi: Arc<FfiRegistry>) -> Self {
        Self {
            ffi,
            script: ScriptRegistry::new(),
            namespace_path: Vec::new(),
            imported_namespaces: Vec::new(),
        }
    }

    // === Name Resolution ===

    /// Build qualified name from current namespace
    pub fn qualified_name(&self, name: &str) -> String {
        if self.namespace_path.is_empty() {
            name.to_string()
        } else {
            format!("{}::{}", self.namespace_path.join("::"), name)
        }
    }

    /// Resolve type name to TypeHash using namespace rules
    pub fn resolve_type(&self, name: &str) -> Result<TypeHash, SemanticError> {
        // 1. Try primitive
        if let Some(hash) = primitive_hash_from_name(name) {
            return Ok(hash);
        }

        // 2. Try current namespace
        let qualified = self.qualified_name(name);
        if let Some(hash) = self.lookup_type(&qualified) {
            return Ok(hash);
        }

        // 3. Try imported namespaces
        for ns in &self.imported_namespaces {
            let qualified = format!("{}::{}", ns, name);
            if let Some(hash) = self.lookup_type(&qualified) {
                return Ok(hash);
            }
        }

        // 4. Try global
        if let Some(hash) = self.lookup_type(name) {
            return Ok(hash);
        }

        Err(SemanticError::unknown_type(name))
    }

    /// Resolve AST TypeExpr to DataType
    pub fn resolve_type_expr(&self, ty: &TypeExpr) -> Result<DataType, SemanticError> {
        let type_hash = self.resolve_type(ty.base_name())?;

        Ok(DataType {
            type_hash,
            is_const: ty.is_const,
            is_handle: ty.is_handle(),
            is_handle_to_const: ty.is_handle_to_const(),
            ref_modifier: ty.ref_modifier(),
        })
    }

    pub fn enter_namespace(&mut self, name: &str) {
        self.namespace_path.push(name.to_string());
    }

    pub fn exit_namespace(&mut self) {
        self.namespace_path.pop();
    }

    pub fn add_import(&mut self, ns: &str) {
        self.imported_namespaces.push(ns.to_string());
    }

    // === Registry Access ===

    pub fn lookup_type(&self, name: &str) -> Option<TypeHash> {
        self.ffi.lookup_type(name)
            .or_else(|| self.script.lookup_type(name))
    }

    pub fn register_type(&mut self, typedef: TypeDef) -> TypeHash {
        self.script.register_type(typedef)
    }

    pub fn register_function(&mut self, func: FunctionDef<'ast>) -> TypeHash {
        self.script.register_function(func)
    }
}
```

### Pass 1: RegistrationPass

**File:** `crates/angelscript-compiler/src/passes/registration.rs`

Single AST walk, complete signatures:

```rust
pub struct RegistrationPass<'ast, 'ctx> {
    context: &'ctx mut CompilationContext<'ast>,
    errors: Vec<SemanticError>,
}

impl<'ast, 'ctx> RegistrationPass<'ast, 'ctx> {
    pub fn run(context: &'ctx mut CompilationContext<'ast>, items: &[Item<'ast>]) -> Vec<SemanticError> {
        let mut pass = Self { context, errors: Vec::new() };

        // Phase 1a: Register all type names first
        for item in items {
            pass.register_type_names(item);
        }

        // Phase 1b: Register all functions with complete signatures
        for item in items {
            pass.register_functions(item);
        }

        // Phase 1c: Validate inheritance/interfaces
        pass.validate_type_relationships();

        pass.errors
    }

    fn register_function(&mut self, func: &FunctionDecl<'ast>, object_type: Option<TypeHash>) {
        // Resolve params using context's name resolution
        let params: Vec<ScriptParam> = func.params.iter()
            .filter_map(|p| {
                let data_type = self.context.resolve_type_expr(&p.ty)
                    .map_err(|e| self.errors.push(e))
                    .ok()?;
                Some(ScriptParam::new(p.name.name, data_type))
            })
            .collect();

        // Resolve return type
        let return_type = func.return_type
            .and_then(|rt| self.context.resolve_type_expr(&rt.ty).ok())
            .unwrap_or_else(|| DataType::void());

        // Compute func_hash from RESOLVED type hashes
        let param_hashes: Vec<TypeHash> = params.iter()
            .map(|p| p.data_type.type_hash)
            .collect();

        let func_hash = self.compute_func_hash(func, object_type, &param_hashes);

        // Register COMPLETE function
        let func_def = FunctionDef {
            func_hash,
            name: func.name.name.to_string(),
            namespace: self.context.namespace_path().to_vec(),
            params,          // COMPLETE
            return_type,     // RESOLVED
            object_type,
            traits: self.build_traits(func),
            is_native: false,
            visibility: func.visibility.into(),
        };

        self.context.register_function(func_def);

        // Add to class methods/behaviors
        if let Some(type_id) = object_type {
            self.context.add_method_to_class(type_id, func_hash);
        }
    }
}
```

### Pass 2: Compilation (Split Components)

**Directory:** `crates/angelscript-compiler/src/passes/compilation/`

Type checking + bytecode generation, split into independently testable components:

#### Orchestrator (`mod.rs`)

```rust
pub struct CompilationPass<'ast, 'ctx> {
    context: &'ctx CompilationContext<'ast>,
    errors: Vec<SemanticError>,
}

impl<'ast, 'ctx> CompilationPass<'ast, 'ctx> {
    pub fn run(context: &'ctx CompilationContext<'ast>, items: &[Item<'ast>]) -> CompilationOutput {
        let mut pass = Self::new(context);

        for item in items {
            pass.compile_item(item);
        }

        CompilationOutput {
            bytecode: pass.bytecode.finish(),
            errors: pass.errors,
        }
    }
}
```

#### ExpressionChecker (`expr_checker.rs`)

```rust
/// Independently testable expression type checker
pub struct ExpressionChecker<'a, 'ast> {
    context: &'a CompilationContext<'ast>,
    local_scope: &'a LocalScope,
    current_class: Option<TypeHash>,
}

impl<'a, 'ast> ExpressionChecker<'a, 'ast> {
    pub fn check(&mut self, expr: &'ast Expr<'ast>) -> Result<ExprInfo, SemanticError>;

    // Delegates to specialized checkers
    fn check_call(&mut self, call: &CallExpr) -> Result<ExprInfo, SemanticError> {
        CallChecker::new(self.context, self.local_scope).check(call)
    }

    fn check_binary_op(&mut self, op: &BinaryExpr) -> Result<ExprInfo, SemanticError> {
        OperatorChecker::new(self.context).check_binary(op, self.check(op.left)?, self.check(op.right)?)
    }
}
```

#### OverloadResolver (`overload.rs`)

```rust
/// Function overload resolution - fully unit testable
pub struct OverloadResolver<'a, 'ast> {
    context: &'a CompilationContext<'ast>,
}

impl<'a, 'ast> OverloadResolver<'a, 'ast> {
    pub fn resolve(
        &self,
        candidates: &[TypeHash],  // func_hashes
        arg_types: &[DataType],
    ) -> Result<TypeHash, OverloadError>;

    fn score_candidate(&self, func: &FunctionDef, args: &[DataType]) -> Option<u32>;
}
```

#### ExprInfo (renamed from ExprContext)

```rust
/// Result of expression type checking (Task 21 naming: *Info for compile-time metadata)
#[derive(Debug, Clone)]
pub struct ExprInfo {
    pub data_type: DataType,
    pub is_lvalue: bool,
    pub is_constant: bool,
    // ... other metadata
}
```

---

## Rust Idiom Improvements

### Make DataType Copy

Currently cloned 175+ times. All fields can be Copy:

```rust
// crates/angelscript-compiler/src/types/data_type.rs
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DataType {
    pub type_hash: TypeHash,      // Copy (u64)
    pub is_const: bool,           // Copy
    pub is_handle: bool,          // Copy
    pub is_handle_to_const: bool, // Copy
    pub ref_modifier: RefModifier, // Copy (enum)
}
```

### Add Display Traits

```rust
impl Display for DataType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // e.g., "const Player@" or "int&"
    }
}

impl Display for TypeHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "TypeHash(0x{:016x})", self.0)
    }
}
```

---

## Naming Conventions

Apply consistently across new crates:

| Pattern | Usage | Example |
|---------|-------|---------|
| `*Pass` | Compiler passes | `RegistrationPass`, `CompilationPass` |
| `*Output` | Pass results | `RegistrationOutput`, `CompilationOutput` |
| `*Info` | Compile-time metadata | `ExprInfo` (not ExprContext) |
| `*Scope` | Symbol scoping | `LocalScope` |
| `get_*` | By ID, assumes exists | `get_type(hash)` |
| `lookup_*` | By name, returns Option | `lookup_type(name)` |
| `find_*` | Complex resolution | `find_compatible_function(...)` |

---

## Files to Delete (after migration)

| File | Lines |
|------|-------|
| `src/semantic/passes/registration.rs` | 967 |
| `src/semantic/passes/type_compilation.rs` | 3,397 |
| `src/ast/visitor.rs` | 1,805 |
| Parts of `src/semantic/types/registry.rs` | ~500 |
| Parts of `src/semantic/passes/function_processor/` | ~8,000 |

**Net reduction**: ~8,000+ lines removed (after factoring out reusable code)

---

## Benefits

1. **Clean slate** - No legacy constraints or backwards compatibility
2. **2 passes instead of 3** - Faster compilation
3. **No format!() overhead** - Proper type resolution
4. **Task 21 progress** - Workspace structure, split components, naming consistency
5. **Better testability** - Independent components (OverloadResolver, ExpressionChecker, etc.)
6. **Side-by-side testing** - Can verify correctness before switching
7. **Easier to understand** - Fresh, well-organized code with consistent naming
8. **DataType as Copy** - Eliminates 175+ clone() calls
9. **~8,000 lines deleted** - visitor.rs, type_compilation.rs, registration.rs, function_processor parts
10. **Display traits** - Better error messages and debugging

---

## Verification

```bash
# Build workspace
cargo build --workspace

# Test both compilers
cargo test --lib
cargo test --lib --features new_compiler

# Benchmark comparison
cargo bench --bench module_benchmarks
cargo bench --bench module_benchmarks --features new_compiler
```

Expected: New compiler matches or exceeds old performance, with cleaner code.
