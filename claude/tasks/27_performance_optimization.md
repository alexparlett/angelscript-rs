# Task 27: Performance Optimization

## Status: In Progress

This task was identified during the FFI/compiler refactor (Task 26). Review after crate restructuring is complete.

---

## Current Status

- **Baseline:** 2.28ms (before arena allocation)
- **After arena:** 1.31ms (73-75% improvement)
- **Target:** <1ms (need ~24% more improvement)
- **FFI overhead:** ~4ms additional (identified root causes below)

---

## Phase A: FFI Performance Fixes (~4ms recovery)

### A1: Fix O(n²) in FfiRegistryBuilder.build() - ✅ COMPLETE

**File:** `crates/angelscript-ffi/src/registry/ffi_registry.rs`
**Lines:** ~1011-1028

**Problem:**
```rust
let behaviors: FxHashMap<TypeHash, TypeBehaviors> = self.behaviors
    .into_iter()
    .filter_map(|(type_id, behaviors)| {
        types.values()                    // O(n) for each behavior
            .find(|def| def.type_hash() == type_id)  // O(n) search
            .map(|def| (def.type_hash(), behaviors))
    })
    .collect();
```

**Fix:** Use `type_id` directly (it's already the hash):
```rust
let behaviors: FxHashMap<TypeHash, TypeBehaviors> = self.behaviors
    .into_iter()
    .filter(|(type_id, _)| types.contains_key(type_id))
    .collect();
```

**Impact:** 1-2ms

---

### A2: Cache type_by_name in FfiRegistry

**File:** `crates/angelscript-ffi/src/registry/ffi_registry.rs`
**Lines:** ~146-151

**Problem:**
```rust
pub fn type_by_name(&self) -> FxHashMap<String, TypeHash> {
    self.types.iter()
        .map(|(hash, def)| (def.qualified_name().to_string(), *hash))
        .collect()
}
```

Called from CompilationContext init (once per Unit). Allocates string for every FFI type.

**Fix:** Cache this map in FfiRegistry at build time as a field.

**Impact:** 0.5-1ms

---

### A3: Return reference from func_by_name()

**File:** `crates/angelscript-ffi/src/registry/ffi_registry.rs`
**Lines:** ~224-232

**Problem:**
```rust
pub fn func_by_name(&self) -> FxHashMap<String, Vec<TypeHash>> {
    self.function_overloads.iter()
        .map(|(name, hashes)| (name.clone(), hashes.clone()))
        .collect()
}
```

**Fix:** Return `&FxHashMap<String, Vec<TypeHash>>` reference instead of cloning.

**Impact:** 0.5-1ms

---

### A4: Store Arc refs in CompilationContext

**File:** `src/semantic/compilation_context.rs`
**Lines:** ~258-259

**Problem:**
```rust
type_by_name: ffi.type_by_name().clone(),
func_by_name: ffi.func_by_name().clone(),
```

**Fix:** Store Arc references to pre-built maps, not cloned copies.

**Impact:** 0.5-1ms

---

## Phase B: Script Compilation Optimizations (1.31ms → <1ms)

### B1: Lexer Cursor Optimization (10-15%)

**File:** `src/lexer/cursor.rs`

**Problem:** `peek()` creates char iterator every call:
```rust
pub fn peek(&self) -> Option<char> {
    self.rest.chars().next()  // Creates iterator every call!
}
```

**Fix A - ASCII fast path in peek():**
```rust
#[inline]
pub fn peek(&self) -> Option<char> {
    let bytes = self.rest.as_bytes();
    let first = *bytes.first()?;
    if first < 128 {
        Some(first as char)  // No iterator creation for ASCII
    } else {
        self.rest.chars().next()  // UTF-8 path unchanged
    }
}
```

**Fix B - Add eat_while_ascii() for identifiers/numbers:**
```rust
#[inline]
pub fn eat_while_ascii(&mut self, f: impl Fn(u8) -> bool) -> &'src str {
    let start = self.offset as usize;
    let bytes = self.rest.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i] < 128 && f(bytes[i]) {
        i += 1;
    }
    self.rest = &self.rest[i..];
    self.offset += i as u32;
    &self.source[start..self.offset as usize]
}
```

Then update `src/lexer/lexer.rs` to use `eat_while_ascii()` for identifiers and numbers.

---

### B2: Cache qualified_name (8%)

**File:** `crates/angelscript-core/src/function_def.rs`

**Problem:**
```rust
pub fn qualified_name(&self) -> String {
    format!("{}::{}", self.namespace.join("::"), self.name)
}
```

Called frequently, creates new String every time.

**Fix:** Cache on first call using OnceCell:
```rust
pub struct FunctionDef {
    cached_qualified_name: OnceCell<String>,
    // ...
}

pub fn qualified_name(&self) -> &str {
    self.cached_qualified_name.get_or_init(|| {
        if self.namespace.is_empty() {
            self.name.clone()
        } else {
            format!("{}::{}", self.namespace.join("::"), self.name)
        }
    })
}
```

---

### B3: Avoid namespace_path Cloning (3-4%)

**Files:**
- `src/semantic/passes/registration.rs`
- `src/semantic/passes/function_processor/mod.rs`

**Problem:** Every function registration clones `Vec<String>` for namespace.

**Fix:** Pass by reference where possible, consider `Rc<[String]>` for FunctionDef storage.

---

### B4: Avoid format!() for TypeHash (2%)

**File:** `src/semantic/passes/registration.rs`

**Problem:**
```rust
let param_hashes: Vec<TypeHash> = func.params.iter()
    .map(|p| TypeHash::from_name(&format!("{}", p.ty)))
    .collect();
```

**Fix:** Add `TypeHash::from_ast_type()` that hashes directly without String allocation.

---

### B5: Remove Redundant types_by_hash (2-4%)

**File:** `src/semantic/types/registry.rs`

**Problem:** `types_by_hash: FxHashMap<TypeHash, TypeHash>` maps hash to itself.

**Fix:** Delete entirely, use `types.contains_key(&hash)` instead.

---

### B6: Unit.clear() API

**File:** `src/unit.rs`

Add ability to reset Unit for recompilation without full recreation:
```rust
impl Unit {
    pub fn clear(&mut self) {
        self.script_registry.clear();
        self.compiled = false;
    }
}
```

Enables more accurate benchmarks and hot reload.

---

### B7: Benchmark Restructuring

**File:** `benches/module_benchmarks.rs`

Add `unit/build_only` benchmark that measures just compilation without Unit creation/drop overhead.

---

## Implementation Order

### Phase A: FFI Fixes (do first - recover 4ms)

| # | Task | Est. Impact | Effort | Status |
|---|------|-------------|--------|--------|
| A1 | Fix O(n²) in build() | 1-2ms | Easy | ✅ Complete |
| A2 | Cache type_by_name | 0.5-1ms | Medium | Pending |
| A3 | Return ref from func_by_name | 0.5-1ms | Easy | Pending |
| A4 | Arc refs in CompilationContext | 0.5-1ms | Medium | Pending |

### Phase B: Script Optimizations (1.31ms → <1ms)

| # | Task | Est. Impact | Effort |
|---|------|-------------|--------|
| B1 | Lexer cursor optimization | 10-15% | Medium |
| B2 | Cache qualified_name | 8% | Easy |
| B3 | Avoid namespace_path cloning | 3-4% | Medium |
| B4 | Avoid format!() for TypeHash | 2% | Easy |
| B5 | Remove types_by_hash | 2-4% | Easy |
| B6 | Unit.clear() API | N/A | Easy |
| B7 | Benchmark restructuring | N/A | Easy |

---

## Files to Modify

### Phase A
- `crates/angelscript-ffi/src/registry/ffi_registry.rs`
- `src/semantic/compilation_context.rs`

### Phase B
- `src/lexer/cursor.rs`
- `src/lexer/lexer.rs`
- `src/semantic/types/registry.rs`
- `src/semantic/passes/registration.rs`
- `src/semantic/passes/function_processor/mod.rs`
- `crates/angelscript-core/src/function_def.rs`
- `crates/angelscript-core/src/type_hash.rs`
- `src/unit.rs`
- `benches/module_benchmarks.rs`

---

## Verification

After each change:
```bash
cargo bench --bench module_benchmarks -- stress_5000
```

Compare times and run profiler to confirm hotspot reduction.

---

## Dependencies

- Task 26 (Compiler Rewrite) should be completed first
- FFI crate restructuring may affect file paths above
- Some fixes (qualified_name caching) depend on angelscript-core crate existing
