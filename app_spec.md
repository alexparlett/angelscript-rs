# AngelScript-Rust Architecture Specification

## Overview

A complete Rust implementation of an AngelScript scripting engine. The project provides parsing, compilation, and runtime support for the AngelScript language.

## Crate Architecture

```
angelscript (root)
├── angelscript-core       # Core types, IDs, spans, type definitions
├── angelscript-parser     # Lexer, AST, visitor pattern
├── angelscript-registry   # Type registry, module system
├── angelscript-compiler   # Bytecode generation, type resolution
├── angelscript-macros     # Procedural macros for registration
└── angelscript-modules    # Standard library (math, string, array, dictionary)
```

## Dependency Graph

```
angelscript-core (no deps)
       ↓
angelscript-parser (core)
       ↓
angelscript-registry (core)
       ↓
angelscript-compiler (core, parser, registry)
       ↓
angelscript-macros (standalone proc-macro)
       ↓
angelscript-modules (core, registry, macros)
       ↓
angelscript (all crates)
```

## Core Components

### angelscript-core
- **Span/IDs**: Source location tracking, type/function identifiers
- **Type System**: DataType, TypeDef, behaviors, operators
- **Entries**: TypeEntry, FunctionEntry, ClassEntry, InterfaceEntry, EnumEntry
- **Traits**: Any, Convert, NativeFn
- **Hash**: Type hashing for templates

### angelscript-parser
- **Lexer**: Tokenization with cursor-based iteration
- **AST**: Expression, Statement, Declaration nodes
- **Parser**: Recursive descent parser with error recovery
- **Visitor**: Zero-cost traversal abstraction

### angelscript-registry
- **Registry**: Central store for all type entries
- **Module**: Script module compilation unit

### angelscript-compiler
- **Context**: Compilation state, scope management
- **Type Resolver**: Resolves AST types to registry entries
- **Bytecode**: Opcode definitions, chunk building, constants
- **Overload**: Function overload resolution with ranking
- **Template**: Generic instantiation and validation
- **Conversion**: Implicit/explicit type conversions
- **Passes**: Multi-pass compilation (registration, etc.)
- **Emit**: Bytecode emission with jump handling

### angelscript-macros
- **derive_any**: Derive macro for Any trait
- **function**: Register Rust functions to AngelScript
- **interface**: Define AngelScript interfaces
- **funcdef**: Function definition type macros

### angelscript-modules
- **math**: Trigonometry, power, rounding, etc.
- **string**: String type and operations
- **array**: Template array type
- **dictionary**: Key-value dictionary

## Key Design Decisions

1. **Arena Allocation**: Uses bumpalo for AST nodes (zero-copy parsing)
2. **Span-Based**: AST nodes store spans, not copies of source text
3. **Workspace Crates**: Modular design with clear separation
4. **Edition 2024**: Uses latest Rust edition features
5. **Zero-Cost Abstractions**: Visitor pattern without runtime overhead
6. **Type Hashing**: xxhash for fast template instantiation lookup

## Build Configuration

- **Profiling**: Optional profiling with puffin support
- **Benchmarks**: Criterion-based benchmarks
- **Dev**: cargo-husky for pre-commit hooks (rustfmt)

## Test Structure

- Unit tests: Per-crate `#[cfg(test)]` modules
- Integration tests: `tests/unit_tests.rs`
- Trybuild tests: Compile-fail tests for macros
