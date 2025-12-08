# Task 46: Compiler API & Validation

## Overview

Wire up the complete compilation pipeline, expose the public API, and validate that all existing tests pass with acceptable performance.

## Goals

1. Wire up complete compilation pipeline
2. Expose clean public API (`Compiler`, `CompiledModule`)
3. All existing tests passing (`cargo test --lib`, `cargo test --test test_harness`, `cargo test --test module_tests`)
4. Performance benchmarks pass (`cargo bench`)
5. Document compiler API

## Dependencies

- All previous tasks (31-45)
- angelscript-ffi crate (for FFI types)
- Existing test infrastructure
- Existing benchmark infrastructure (`benches/module_benchmarks.rs`)

## Files to Create/Modify

```
crates/angelscript-compiler/src/
├── lib.rs                        # Public API exports
├── compiler.rs                   # Main Compiler struct
└── pipeline.rs                   # Compilation pipeline
```

## Detailed Implementation

### Main Compiler API (lib.rs)

```rust
//! AngelScript Compiler
//!
//! Two-pass compiler that transforms AngelScript source into bytecode.
//!
//! # Example
//!
//! ```rust
//! use angelscript_compiler::{Compiler, CompilerOptions};
//! use angelscript_ffi::Context;
//!
//! let mut ctx = Context::with_default_modules();
//! let mut compiler = Compiler::new(&mut ctx);
//!
//! let module = compiler.compile("test", r#"
//!     int add(int a, int b) { return a + b; }
//! "#)?;
//! ```

mod bytecode;
mod compiler;
mod context;
mod conversion;
mod error;
mod expr;
mod function_compiler;
mod overload;
mod pass;
mod pipeline;
mod resolution;
mod scope;
mod stmt;
mod template;

pub use bytecode::{BytecodeChunk, OpCode};
pub use compiler::{Compiler, CompilerOptions};
pub use error::{CompileError, CompileResult};
pub use pipeline::CompiledModule;

// Re-export core types used in API
pub use angelscript_core::{DataType, TypeHash, Span};
```

### Compiler Struct (compiler.rs)

```rust
use angelscript_core::UnitId;
use angelscript_ffi::Context;
use angelscript_parser::Parser;

use crate::context::CompilationContext;
use crate::error::{CompileError, CompileResult};
use crate::pass::{RegistrationPass, CompilationPass};
use crate::pipeline::CompiledModule;

/// Compiler options.
#[derive(Debug, Clone, Default)]
pub struct CompilerOptions {
    /// Enable optimizations.
    pub optimize: bool,
    /// Generate debug info.
    pub debug_info: bool,
    /// Strict mode (warnings become errors).
    pub strict: bool,
}

/// Main compiler interface.
pub struct Compiler<'ctx> {
    ctx: CompilationContext<'ctx>,
    options: CompilerOptions,
}

impl<'ctx> Compiler<'ctx> {
    /// Create a new compiler using the given FFI context.
    pub fn new(ffi_ctx: &'ctx mut Context) -> Self {
        Self {
            ctx: CompilationContext::new(ffi_ctx.registry_mut()),
            options: CompilerOptions::default(),
        }
    }

    /// Create with custom options.
    pub fn with_options(ffi_ctx: &'ctx mut Context, options: CompilerOptions) -> Self {
        Self {
            ctx: CompilationContext::new(ffi_ctx.registry_mut()),
            options,
        }
    }

    /// Compile source code into a module.
    pub fn compile(&mut self, name: &str, source: &str) -> CompileResult<CompiledModule> {
        let unit_id = self.ctx.create_unit(name);

        // Parse
        let ast = Parser::new(source).parse()
            .map_err(|e| CompileError::ParseError {
                message: e.to_string(),
                span: e.span(),
            })?;

        // Pass 1: Registration
        let mut registration = RegistrationPass::new(&mut self.ctx, unit_id);
        registration.run(&ast)?;

        // Pass 2: Compilation
        let mut compilation = CompilationPass::new(&mut self.ctx, unit_id);
        let output = compilation.run(&ast)?;

        Ok(CompiledModule {
            name: name.to_string(),
            unit_id,
            bytecode: output.bytecode,
            global_inits: output.global_inits,
        })
    }

    /// Get access to compilation context.
    pub fn context(&self) -> &CompilationContext<'ctx> {
        &self.ctx
    }
}
```

### Compiled Module (pipeline.rs)

```rust
use angelscript_core::{TypeHash, UnitId};
use rustc_hash::FxHashMap;

use crate::bytecode::BytecodeChunk;

/// A compiled module containing bytecode for all functions.
#[derive(Debug)]
pub struct CompiledModule {
    pub name: String,
    pub unit_id: UnitId,
    pub bytecode: FxHashMap<TypeHash, BytecodeChunk>,
    pub global_inits: Vec<TypeHash>,
}

impl CompiledModule {
    /// Get bytecode for a specific function.
    pub fn get_function(&self, hash: TypeHash) -> Option<&BytecodeChunk> {
        self.bytecode.get(&hash)
    }

    /// Get all function hashes in this module.
    pub fn functions(&self) -> impl Iterator<Item = TypeHash> + '_ {
        self.bytecode.keys().copied()
    }

    /// Total bytecode size.
    pub fn bytecode_size(&self) -> usize {
        self.bytecode.values().map(|c| c.code.len()).sum()
    }
}
```

## Validation

### All Tests Must Pass

```bash
# Unit tests (~2400+)
cargo test --lib

# Parser integration tests
cargo test --test test_harness

# Module/runtime integration tests
cargo test --test module_tests

# Compiler crate tests
cargo test -p angelscript-compiler
```

### Performance Benchmarks

```bash
# All benchmarks
cargo bench

# Specific groups
cargo bench -- "unit/file_sizes"      # Tiny to stress-test (5-5000 lines)
cargo bench -- "unit/features"        # Functions, classes, expressions
cargo bench -- "unit/real_world"      # Game logic, utilities
cargo bench -- "unit/complexity"      # Complexity-based tests
```

**Performance targets:**
- Small files (<100 lines): < 1ms compile time
- Medium files (100-500 lines): < 5ms compile time
- Large files (500-2000 lines): < 20ms compile time
- Stress tests (5000 lines): < 100ms compile time

### Test Scripts Coverage

The existing `test_scripts/` directory covers:
- `hello_world.as` - Basic function
- `literals.as` - Numeric, string, bool literals
- `operators.as` - Arithmetic, comparison, logical
- `control_flow.as` - if/else, while, for, switch
- `functions.as` - Function declarations, overloads
- `classes.as` - Class definitions, methods
- `interfaces.as` - Interface implementation
- `inheritance.as` - Class inheritance
- `handles.as` - Handle (@) semantics
- `arrays.as` - array<T> usage
- `templates.as` - Template instantiation
- `enums.as` - Enum definitions
- `namespaces.as` - Namespace scoping
- `properties.as` - Property getters/setters
- `operators_overload.as` - Operator overloading
- `performance/*.as` - Performance test scripts

## Acceptance Criteria

- [ ] `Compiler::compile()` produces `CompiledModule`
- [ ] `cargo test --lib` passes (all unit tests)
- [ ] `cargo test --test test_harness` passes
- [ ] `cargo test --test module_tests` passes
- [ ] `cargo bench` runs without regression
- [ ] Performance meets targets above
- [ ] Public API is documented
- [ ] Errors include source spans

## Summary

This completes the compiler task breakdown. The 16 tasks cover:

| Task | Name | Description |
|------|------|-------------|
| 31 | Compiler Foundation | Core types, bytecode, opcodes |
| 32 | Compilation Context | Unified type lookup |
| 33 | Type Resolution | TypeExpr → DataType |
| 34 | Template Instantiation | Template types, functions, cache |
| 35 | Conversion System | Type conversions |
| 36 | Overload Resolution | Function selection |
| 37 | Registration Pass | Pass 1 - declarations |
| 38 | Local Scope | Variable tracking |
| 39 | Bytecode Emitter | Instruction generation |
| 40 | Expression Basics | Literals, operators |
| 41 | Expression Calls | Function/method calls |
| 42 | Expression Advanced | Cast, lambda, ternary |
| 43 | Statement Basics | Blocks, if, while |
| 44 | Statement Loops | For, foreach, switch |
| 45 | Function Compilation | Pass 2 orchestration |
| 46 | Compiler API | Public API, integration |

Each task is independently implementable and can be committed incrementally.
