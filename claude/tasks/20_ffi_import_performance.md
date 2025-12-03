# Task 20: FFI Import Performance Optimization

## Problem Statement

We've observed a significant performance regression in the benchmarks. A test that used to take ~100μs is now taking ~2.2ms - approximately a 20x slowdown.

The root cause is that FFI module import (`import_modules`) happens on every compilation, not once at context creation time. Every call to `Unit::build()` creates a new `Registry` and re-imports all FFI modules from scratch.

## Current Flow

```
Context::with_default_modules()
  -> Creates Module objects (cheap)
  -> Stores them in Context

Unit::build()
  -> Compiler::compile_with_modules(script, modules)
    -> Registry::new()  // Fresh registry every time
    -> registry.import_modules(modules)  // EXPENSIVE - happens every build!
      -> Phase 1: Register enums
      -> Phase 2: Register interfaces
      -> Phase 3: Register funcdefs
      -> Phase 4: Register type shells
      -> Phase 5: Fill type details (methods, operators, properties)
      -> Phase 6: Register global functions
      -> Phase 7: Register global properties
```

## Why This Is Expensive

The `import_modules` function does a lot of work:
1. String allocations for qualified names
2. Type resolution (parsing type strings, looking up types)
3. Method/operator/property conversion
4. HashMap insertions for type lookups
5. Cloning of method signatures, parameters, etc.

This happens for ALL 5 default modules (std, string, math, array, dictionary) on every single compilation.

## Potential Solutions

### Option A: Cache Registry in Context

Pre-compute the registry at context creation time and clone it for each compilation.

**Pros:**
- Registry import happens once
- Simple conceptually

**Cons:**
- Registry has lifetime parameters tied to AST ('ast)
- Would need to make Registry cloneable (may be expensive)
- Ownership issues with sharing registry between compilations

### Option B: Make Registry Cloneable and Cache

Similar to A, but implement efficient cloning (possibly copy-on-write or Arc-based sharing).

**Pros:**
- Could share immutable FFI data between compilations

**Cons:**
- Complex implementation
- Need to separate "base" FFI data from "script" data

### Option C: Two-Tier Registry

Split registry into:
1. Base registry (FFI types, immutable, shared via Arc)
2. Script registry (script types, mutable, per-compilation)

**Pros:**
- Clean separation of concerns
- FFI data is truly shared
- No cloning needed

**Cons:**
- Significant refactoring
- Need to handle type ID namespacing

### Option D: Lazy Import / Import on Demand

Only import types/functions when they're first referenced.

**Pros:**
- Only pay for what you use

**Cons:**
- Complex dependency resolution
- May cause performance spikes during compilation
- Harder to report "undefined type" errors

### Option E: Optimize Import Speed

Make the import faster without changing architecture:
- Pre-compute more at registration time
- Reduce string allocations
- Use interned strings
- Avoid cloning where possible

**Pros:**
- No architectural changes
- Incremental improvement

**Cons:**
- May not achieve the full speedup needed
- Treating symptoms not cause

## Recommended Approach

Start with **Option E** (optimize import speed) as a quick win, then consider **Option C** (two-tier registry) for a proper architectural fix.

## Investigation Tasks

1. [ ] Profile `import_modules` to find the hotspots
2. [ ] Measure time breakdown across the 7 phases
3. [ ] Identify which modules are most expensive to import
4. [ ] Prototype optimizations and measure impact

## Success Criteria

- Benchmark times return to ~100μs range (or close to it)
- No regression in functionality
- All tests continue to pass
