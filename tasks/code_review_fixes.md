# Code Review Fixes Plan

## Overview
This plan addresses all issues identified in the comprehensive code review, organized into phases by priority and dependency.

---

## Phase 1: Move Inheritance Resolution to Pass 1b

### 1.1 Defer Inheritance Resolution
**Files:**
- `crates/angelscript-core/src/entries/class.rs` - Add pending inheritance fields
- `crates/angelscript-core/src/entries/interface.rs` - Add pending base interfaces field
- `crates/angelscript-compiler/src/passes/registration.rs` - Remove resolve_inheritance, store names
- `crates/angelscript-compiler/src/passes/completion.rs` - Add inheritance resolution phase
- `crates/angelscript-compiler/src/context.rs` - Remove scope rebuild from register_*

**Problem:**
1. Every `register_type()` call triggers `rebuild_scope()`, resulting in O(n²) complexity
2. Forward references don't work (class declared after use causes UnknownType error)

**Solution:**
Move inheritance name resolution from Pass 1 to Pass 1b. This enables:
- Zero scope lookups during Pass 1 (registration)
- Forward references work automatically
- Single scope rebuild after all types registered

### 1.2 Schema Changes

**New types in `crates/angelscript-compiler/src/passes/mod.rs`:**
```rust
/// Unresolved inheritance reference from AST
#[derive(Debug, Clone)]
pub struct PendingInheritance {
    pub name: String,                    // Raw name from source (e.g., "Base", "Foo::Bar")
    pub span: Span,                      // For error reporting
    pub namespace_context: Vec<String>,  // Current namespace when parsed
    pub imports: Vec<String>,            // Active imports when parsed
}

/// Pending resolutions collected during Pass 1, consumed by Pass 1b
#[derive(Debug, Default)]
pub struct PendingResolutions {
    /// Class inheritance: class_hash -> pending bases/mixins/interfaces
    pub class_inheritance: FxHashMap<TypeHash, Vec<PendingInheritance>>,
    /// Interface inheritance: interface_hash -> pending base interfaces
    pub interface_bases: FxHashMap<TypeHash, Vec<PendingInheritance>>,
}
```

**No changes to ClassEntry or InterfaceEntry** - they keep their existing resolved fields only.

### 1.3 Pass 1 Changes (registration.rs)

**Remove:** `resolve_inheritance()`, `resolve_mixin_inheritance()`

**Add to `RegistrationPass` struct:**
```rust
pub struct RegistrationPass<'a, 'reg> {
    // ... existing fields
    pending_resolutions: PendingResolutions,
}
```

**Update `RegistrationOutput`:**
```rust
pub struct RegistrationOutput {
    pub types_registered: usize,
    pub functions_registered: usize,
    pub errors: Vec<CompilationError>,
    pub pending_resolutions: PendingResolutions,  // NEW
}
```

**Modify `visit_class()`:**
```rust
fn visit_class(&mut self, class: &ClassDecl<'_>) {
    let name = class.name.name.to_string();
    let qualified_name = self.qualified_name(&name);
    let type_hash = TypeHash::from_name(&qualified_name);

    // Collect pending inheritance (don't resolve yet)
    if !class.inheritance.is_empty() {
        let pending: Vec<PendingInheritance> = class.inheritance
            .iter()
            .map(|expr| PendingInheritance {
                name: self.ident_expr_to_string(expr),
                span: expr.span,
                namespace_context: self.current_namespace_vec(),
                imports: self.ctx.imports().to_vec(),
            })
            .collect();
        self.pending_resolutions.class_inheritance.insert(type_hash, pending);
    }

    // Create class entry WITHOUT inheritance (will be set in Pass 1b)
    let class_entry = ClassEntry::new(/* ... */);

    // Register without resolving inheritance
    self.ctx.register_type(class_entry.into())?;

    // ... rest of member registration
}
```

**Modify `visit_interface()`:** Similar - store pending base interface names in `pending_resolutions.interface_bases`.

### 1.4 Pass 1b Changes (completion.rs)

**Update `TypeCompletionPass` to accept pending resolutions:**

```rust
pub struct TypeCompletionPass<'reg> {
    registry: &'reg mut SymbolRegistry,
    pending: PendingResolutions,  // NEW
}

impl<'reg> TypeCompletionPass<'reg> {
    pub fn new(registry: &'reg mut SymbolRegistry, pending: PendingResolutions) -> Self {
        Self { registry, pending }
    }

    pub fn run(mut self) -> CompletionOutput {
        let mut output = CompletionOutput::default();

        // NEW: Phase 1 - Resolve all pending inheritance
        self.resolve_all_inheritance(&mut output);
        if !output.errors.is_empty() {
            return output;  // Bail on resolution errors
        }

        // Phase 2 - Topological sort (existing)
        let class_hashes: Vec<TypeHash> = self.registry.classes()
            .map(|c| c.type_hash)
            .collect();
        let ordered = match self.topological_sort(&class_hashes) {
            Ok(ordered) => ordered,
            Err(e) => {
                output.errors.push(e);
                return output;
            }
        };

        // Phase 3 - Copy inherited members (existing)
        for class_hash in ordered {
            self.complete_class(class_hash, &mut output);
        }

        output
    }

    fn resolve_all_inheritance(&mut self, output: &mut CompletionOutput) {
        // Process class inheritance
        for (class_hash, pending_list) in std::mem::take(&mut self.pending.class_inheritance) {
            self.resolve_class_inheritance(class_hash, pending_list, output);
        }

        // Process interface inheritance
        for (iface_hash, pending_list) in std::mem::take(&mut self.pending.interface_bases) {
            self.resolve_interface_inheritance(iface_hash, pending_list, output);
        }
    }

    fn resolve_class_inheritance(
        &mut self,
        class_hash: TypeHash,
        pending_list: Vec<PendingInheritance>,
        output: &mut CompletionOutput,
    ) {
        let class_name = self.registry.get(class_hash)
            .and_then(|e| e.as_class())
            .map(|c| c.name.clone())
            .unwrap_or_default();

        let mut base_class = None;
        let mut mixins = Vec::new();
        let mut interfaces = Vec::new();

        for pending in &pending_list {
            // Resolve the type name using namespace context
            let resolved_hash = self.resolve_type_name(pending);

            let Some(hash) = resolved_hash else {
                output.errors.push(CompilationError::UnknownType {
                    name: pending.name.clone(),
                    span: pending.span,
                });
                continue;
            };

            let Some(entry) = self.registry.get(hash) else { continue };

            if entry.is_interface() {
                interfaces.push(hash);
            } else if let Some(class_entry) = entry.as_class() {
                if class_entry.is_mixin {
                    mixins.push(hash);
                } else if base_class.is_none() {
                    // Validate inheritance rules
                    if class_entry.source.is_ffi() {
                        output.errors.push(CompilationError::InvalidOperation {
                            message: format!(
                                "script class '{}' cannot extend FFI class '{}'",
                                class_name, pending.name
                            ),
                            span: pending.span,
                        });
                        continue;
                    }
                    if class_entry.is_final {
                        output.errors.push(CompilationError::InvalidOperation {
                            message: format!(
                                "class '{}' cannot extend final class '{}'",
                                class_name, pending.name
                            ),
                            span: pending.span,
                        });
                        continue;
                    }
                    base_class = Some(hash);
                } else {
                    output.errors.push(CompilationError::Other {
                        message: format!(
                            "multiple inheritance not supported: {} already has a base class",
                            class_name
                        ),
                        span: pending.span,
                    });
                }
            }
        }

        // Update the class entry with resolved inheritance
        if let Some(entry) = self.registry.get_mut(class_hash) {
            if let Some(class) = entry.as_class_mut() {
                class.base_class = base_class;
                class.mixins = mixins;
                class.interfaces = interfaces;
            }
        }
    }

    /// Resolve a type name using stored namespace context
    fn resolve_type_name(&self, pending: &PendingInheritance) -> Option<TypeHash> {
        // 1. Try in current namespace context
        if !pending.namespace_context.is_empty() {
            let qualified = format!("{}::{}", pending.namespace_context.join("::"), pending.name);
            if let Some(entry) = self.registry.get_by_name(&qualified) {
                return Some(entry.type_hash());
            }
        }

        // 2. Try in each imported namespace
        for import in &pending.imports {
            let qualified = format!("{}::{}", import, pending.name);
            if let Some(entry) = self.registry.get_by_name(&qualified) {
                return Some(entry.type_hash());
            }
        }

        // 3. Try in global namespace
        self.registry.get_by_name(&pending.name).map(|e| e.type_hash())
    }
}
```

### 1.5 Context Changes

**Remove `rebuild_scope()` calls from `register_*` methods:**
```rust
pub fn register_type(&mut self, entry: TypeEntry) -> Result<(), RegistrationError> {
    self.unit_registry.register_type(entry)?;
    // REMOVED: self.rebuild_scope();
    Ok(())
}

pub fn register_function(&mut self, entry: FunctionEntry) -> Result<(), RegistrationError> {
    self.unit_registry.register_function(entry)?;
    // REMOVED: self.rebuild_scope();
    Ok(())
}
```

**Add explicit `rebuild_scope()` call after Pass 1 completes** (in unit.rs or wherever passes are orchestrated).

### 1.6 Testing

- All existing inheritance tests should pass
- Add forward reference tests:
  ```angelscript
  class Derived : Base {}
  class Base {}
  ```
- Add cross-namespace forward reference tests
- Add benchmark comparing Pass 1 performance before/after

---

## Phase 2: Code Duplication Removal

### 2.1 Extract Shared `is_type_derived_from` Utility
**Files:**
- `crates/angelscript-compiler/src/context.rs` (add method)
- `crates/angelscript-compiler/src/conversion/mod.rs` (remove duplicate)
- `crates/angelscript-compiler/src/overload/ranking.rs` (remove duplicate)

**Problem:** Two nearly identical implementations of hierarchy walking.

**Solution:**
1. Add `is_type_derived_from()` method to `CompilationContext`
2. Update both call sites to use the shared method

**Changes:**
```rust
// Add to CompilationContext
pub fn is_type_derived_from(&self, derived: TypeHash, base: TypeHash) -> bool {
    if derived == base {
        return true;
    }

    let entry = self.get_type(derived);
    if let Some(entry) = entry {
        if let Some(class) = entry.as_class() {
            if let Some(base_class) = class.base_class {
                return self.is_type_derived_from(base_class, base);
            }
        }
    }
    false
}
```

---

### 2.2 Consolidate Template Argument Skipping
**Files:** `crates/angelscript-parser/src/ast/expr_parser.rs`

**Problem:** `try_skip_template_args()` (lines 420-457) and `try_skip_template_args_simple()` (lines 785-822) are nearly identical.

**Solution:**
1. Create unified `try_skip_template_args_with_options()` method
2. Add parameters for behavior differences (iteration limit, early termination tokens)
3. Replace both functions with calls to the unified version

**Changes:**
```rust
fn try_skip_template_args_with_options(
    &mut self,
    max_iterations: Option<usize>,
    early_exit_tokens: &[TokenKind],
) -> bool {
    if self.eat(TokenKind::Less).is_none() {
        return false;
    }
    let mut depth = 1;
    let mut iterations = 0;

    while depth > 0 && !self.is_eof() {
        if let Some(max) = max_iterations {
            if iterations >= max {
                return false;
            }
            iterations += 1;
        }

        let kind = self.peek().kind;
        if early_exit_tokens.contains(&kind) {
            return false;
        }

        match kind {
            TokenKind::Less => { depth += 1; self.advance(); }
            TokenKind::Greater => { depth -= 1; self.advance(); }
            TokenKind::GreaterGreater => { depth -= 2; self.advance(); }
            TokenKind::GreaterGreaterGreater => { depth -= 3; self.advance(); }
            _ => { self.advance(); }
        }
    }
    depth == 0
}

fn try_skip_template_args(&mut self) -> bool {
    self.try_skip_template_args_with_options(None, &[])
}

fn try_skip_template_args_simple(&mut self) -> bool {
    self.try_skip_template_args_with_options(
        Some(1000),
        &[TokenKind::LeftParen, TokenKind::LeftBrace, TokenKind::Semicolon]
    )
}
```

---

### 2.3 Remove Identical Span Calculation Branches
**Files:** `crates/angelscript-parser/src/ast/type_parser.rs`

**Problem:** Lines 48-63 have three identical branches calculating `end_span`.

**Solution:** Remove redundant conditions, keep single calculation.

**Before:**
```rust
let end_span = if !suffixes.is_empty() {
    self.buffer.get(self.position.saturating_sub(1)).map(|t| t.span).unwrap_or(start_span)
} else if !template_args.is_empty() {
    self.buffer.get(self.position.saturating_sub(1)).map(|t| t.span).unwrap_or(start_span)
} else {
    self.buffer.get(self.position.saturating_sub(1)).map(|t| t.span).unwrap_or(start_span)
};
```

**After:**
```rust
let end_span = self.buffer
    .get(self.position.saturating_sub(1))
    .map(|t| t.span)
    .unwrap_or(start_span);
```

---

## Phase 3: Template Instantiation Optimization

### 3.1 Create TemplateSnapshot Struct
**Files:** `crates/angelscript-compiler/src/template/instantiation.rs`

**Problem:** 8 consecutive clone operations (lines 69-82) to work around borrow checker.

**Solution:**
1. Create `TemplateSnapshot` struct to hold all needed data
2. Single clone operation when creating snapshot
3. Use snapshot throughout instantiation

**Changes:**
```rust
// New struct
struct TemplateSnapshot {
    name: String,
    params: Vec<TypeHash>,
    source: Option<Span>,
    type_kind: TypeKind,
    base_class: Option<TypeHash>,
    methods: Vec<(String, TypeHash)>,
    properties: Vec<PropertyDef>,
    behaviors: Behaviors,
}

impl TemplateSnapshot {
    fn from_class(class: &ClassEntry) -> Self {
        Self {
            name: class.name.clone(),
            params: class.template_params.clone(),
            source: class.source,
            type_kind: class.type_kind.clone(),
            base_class: class.base_class,
            methods: class.methods.iter()
                .map(|(name, hash)| (name.clone(), *hash))
                .collect(),
            properties: class.properties.clone(),
            behaviors: class.behaviors.clone(),
        }
    }
}
```

---

## Phase 4: Large Function Refactoring

### 4.1 Split `resolve_inheritance()`
**Files:** `crates/angelscript-compiler/src/passes/registration.rs`

**Problem:** Function spans lines 372-474 (102 lines) with deep nesting and duplicated validation.

**Solution:**
1. Extract `validate_base_class()` helper
2. Extract `validate_interface_impl()` helper
3. Simplify main function to orchestration

**New helpers:**
```rust
fn validate_base_class(
    &self,
    class_name: &str,
    base_entry: &TypeEntry,
    base_name: &str,
    span: Span,
) -> Result<(), CompilationError> {
    // Check if FFI class
    if base_entry.is_ffi() {
        return Err(CompilationError::cannot_inherit_ffi_class(base_name, span));
    }
    // Check if final
    if let Some(class) = base_entry.as_class() {
        if class.is_final {
            return Err(CompilationError::cannot_inherit_final(base_name, span));
        }
    }
    Ok(())
}

fn validate_interface_impl(
    &self,
    class_name: &str,
    interface_entry: &TypeEntry,
    interface_name: &str,
    span: Span,
) -> Result<(), CompilationError> {
    // Validation logic
}
```

---

### 4.2 Split `parse_ident_or_constructor()`
**Files:** `crates/angelscript-parser/src/ast/expr_parser.rs`

**Problem:** Function spans lines 608-744 (136 lines) handling multiple concerns.

**Solution:**
1. Extract `parse_scoped_identifier()` - handles scope resolution
2. Extract `parse_constructor_call()` - handles constructor invocation
3. Keep main function as dispatcher

---

### 4.3 Split `instantiate_template_type()`
**Files:** `crates/angelscript-compiler/src/template/instantiation.rs`

**Problem:** 181 lines with 13 numbered steps.

**Solution:**
1. Extract `instantiate_template_methods()` - handles method instantiation loop
2. Extract `instantiate_template_properties()` - handles property substitution
3. Extract `create_instance_entry()` - handles registry entry creation

---

### 4.4 Split `scan_operator()`
**Files:** `crates/angelscript-parser/src/lexer/mod.rs`

**Problem:** 171 lines (479-650) with 40+ match patterns.

**Solution:** Group by operator category:
```rust
fn scan_arithmetic_op(&mut self, first: char) -> Option<TokenKind>
fn scan_comparison_op(&mut self, first: char) -> Option<TokenKind>
fn scan_bitwise_op(&mut self, first: char) -> Option<TokenKind>
fn scan_logical_op(&mut self, first: char) -> Option<TokenKind>
fn scan_assignment_op(&mut self, first: char) -> Option<TokenKind>
```

---

## Phase 5: Error Handling Improvements

### 5.1 Fix Silent Literal Parsing Failures
**Files:** `crates/angelscript-parser/src/ast/expr_parser.rs`

**Problem:** Lines 134-161 use `unwrap_or(0)` hiding parse errors.

**Solution:**
```rust
// Before
let value = token.lexeme.parse::<i64>().unwrap_or(0);

// After
let value = token.lexeme.parse::<i64>().map_err(|_| {
    ParseError::new(
        ParseErrorKind::InvalidLiteral,
        token.span,
        format!("invalid integer literal: {}", token.lexeme),
    )
})?;
```

---

### 5.2 Change `target_type()` to Return Option
**Files:** `crates/angelscript-core/src/type_def.rs`

**Problem:** Lines 666-673 panic on non-conversion operators.

**Solution:**
```rust
pub fn target_type(&self) -> Option<TypeHash> {
    match self {
        OperatorBehavior::OpConv(t)
        | OperatorBehavior::OpImplConv(t)
        | OperatorBehavior::OpCast(t)
        | OperatorBehavior::OpImplCast(t) => Some(*t),
        _ => None,
    }
}
```

**Note:** Update all call sites to handle `Option`.

---

### 5.3 Improve Macro Error Messages
**Files:** `crates/angelscript-macros/src/attrs.rs`

**Problem:** Lines 151-157, 186-192 show generic errors without suggestions.

**Solution:**
```rust
return Err(meta.error(format!(
    "unknown attribute '{}'. Valid attributes are: name, value, pod, reference, scoped, nocount, as_handle, template",
    meta.path.get_ident().map(|i| i.to_string()).unwrap_or_default()
)));
```

---

## Phase 6: Registry Optimizations

### 6.1 Use Existing Namespace Index
**Files:** `crates/angelscript-registry/src/registry.rs`

**Problem:** `types_in_namespace()` (lines 382-399) iterates all types instead of using `types_by_namespace` index.

**Solution:**
```rust
pub fn types_in_namespace<'a>(&'a self, ns: &'a str) -> impl Iterator<Item = &'a TypeEntry> {
    self.types_by_namespace
        .get(ns)
        .into_iter()
        .flat_map(|map| map.values())
        .filter_map(|hash| self.types.get(hash))
}
```

---

### 6.2 Defer String Allocation in Registration
**Files:** `crates/angelscript-registry/src/registry.rs`

**Problem:** Lines 160-164 clone strings before duplicate check.

**Solution:**
```rust
pub fn register_type(&mut self, entry: TypeEntry) -> Result<(), RegistrationError> {
    let hash = entry.type_hash();

    // Check duplicate first (no allocation)
    if self.types.contains_key(&hash) {
        return Err(RegistrationError::DuplicateType(
            entry.qualified_name().to_string()  // Only allocate on error
        ));
    }

    // Now allocate for successful path
    let qualified_name = entry.qualified_name().to_string();
    let simple_name = entry.name().to_string();
    let namespace = entry.namespace().join("::");
    // ... rest of registration
}
```

---

## Phase 7: Code Organization

### 7.1 Unify Constructor Tracking in RegistrationPass
**Files:** `crates/angelscript-compiler/src/passes/registration.rs`

**Problem:** 7 separate `Vec` fields for tracking constructor/destructor state (lines 63-104).

**Solution:**
```rust
// Replace multiple Vecs with unified tracking
#[derive(Default)]
struct ClassTraits {
    has_constructor: bool,
    has_copy_constructor: bool,
    has_op_assign: bool,
    deleted_default_ctor: bool,
    deleted_copy_ctor: bool,
    deleted_op_assign: bool,
}

// In RegistrationPass
class_traits: FxHashMap<TypeHash, ClassTraits>,
```

---

### 7.2 Extract Primitive Type Checking in Macros
**Files:** `crates/angelscript-macros/src/function.rs`

**Problem:** Same primitive type list duplicated at lines 688, 912, 1033.

**Solution:**
```rust
// Add to function.rs or new utils.rs
fn is_rust_primitive(type_str: &str) -> bool {
    matches!(
        type_str,
        "i8" | "i16" | "i32" | "i64"
        | "u8" | "u16" | "u32" | "u64"
        | "isize" | "usize"
        | "f32" | "f64"
        | "bool"
    )
}
```

---

## Phase 8: Documentation

### 8.1 Add Thread Safety Documentation
**Files:** `crates/angelscript-registry/src/registry.rs`

**Add to `SymbolRegistry` struct:**
```rust
/// Unified type and function registry.
///
/// # Thread Safety
///
/// This type is NOT thread-safe. All registration and lookup operations
/// must occur from a single thread. If concurrent access is required,
/// wrap in `Arc<RwLock<SymbolRegistry>>` or `Arc<Mutex<SymbolRegistry>>`.
///
/// The registry is designed for single-threaded compilation followed by
/// read-only access during execution.
pub struct SymbolRegistry { ... }
```

---

### 8.2 Document FFI Contract for VM Implementers
**Files:**
- `crates/angelscript-modules/src/array.rs`
- `crates/angelscript-modules/src/dictionary.rs`

**Add module-level documentation explaining:**
- How VM should implement backing storage
- Memory ownership and reference counting requirements
- Template parameter resolution mechanism

---

## Phase 9: Module Completeness

### 9.1 Mark Incomplete Modules
**Files:**
- `crates/angelscript-modules/src/array.rs`
- `crates/angelscript-modules/src/dictionary.rs`
- `crates/angelscript-modules/src/std.rs`

**Option A - Feature Gate:**
```rust
// In Cargo.toml
[features]
unstable-stdlib = []

// In lib.rs
#[cfg(feature = "unstable-stdlib")]
pub mod array;
```

**Option B - Doc Hidden:**
```rust
#[doc(hidden)]
pub mod array;
```

**Option C - Compile-time Warning:**
```rust
#[deprecated(note = "ScriptArray is not yet implemented - methods will panic")]
pub struct ScriptArray;
```

---

## Implementation Order

| Phase | Effort | Impact | Dependencies |
|-------|--------|--------|--------------|
| 1 (Batch Scope) | 2h | Critical | None |
| 2.3 (Span Fix) | 15m | Low | None |
| 2.1 (is_derived_from) | 1h | Medium | None |
| 2.2 (Template Skip) | 1h | Medium | None |
| 3 (TemplateSnapshot) | 2h | High | None |
| 5.1 (Literal Errors) | 1h | Medium | None |
| 5.2 (target_type Option) | 1h | Medium | None |
| 5.3 (Macro Errors) | 30m | Low | None |
| 6.1 (Namespace Index) | 30m | Medium | None |
| 6.2 (Defer Allocation) | 30m | Low | None |
| 7.1 (ClassTraits) | 2h | Medium | None |
| 7.2 (Primitive Check) | 30m | Low | None |
| 4.1 (resolve_inheritance) | 2h | Medium | Phase 2.1 |
| 4.2 (parse_ident_or_constructor) | 2h | Medium | None |
| 4.3 (instantiate_template) | 2h | Medium | Phase 3 |
| 4.4 (scan_operator) | 1h | Low | None |
| 8 (Documentation) | 1h | Low | None |
| 9 (Module Marking) | 30m | Medium | None |

**Total Estimated Effort:** ~20 hours

---

## Testing Strategy

For each phase:
1. Run existing test suite before changes
2. Make changes incrementally
3. Run test suite after each significant change
4. Add new tests for:
   - Batch registration performance
   - Literal parsing error cases
   - Edge cases in refactored functions

```bash
# Run full test suite
cargo test --workspace

# Run specific crate tests
cargo test -p angelscript-compiler
cargo test -p angelscript-parser

# Run with all features
cargo test --workspace --all-features
```

---

## Success Criteria

- [ ] All existing tests pass
- [ ] No new clippy warnings
- [ ] Batch registration benchmark shows >10x improvement
- [ ] No `todo!()` panics in user-facing code paths (or properly gated)
- [ ] All large functions split to <80 lines
- [ ] No duplicate utility functions
- [ ] Thread safety documented
