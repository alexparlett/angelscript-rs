# Task 28: Unified AngelScript Error Types

## Problem Summary

The codebase currently has 14+ different error types scattered across 5 crates:
- **angelscript-parser**: `LexerError`, `ParseError`, `ParseErrorKind`, `ParseErrors`
- **angelscript-ffi**: `ConversionError`, `NativeError`, `FfiRegistryError`
- **angelscript-compiler**: `RegistrationError`, `ResolutionError`
- **angelscript-module**: `ModuleError`
- **Main crate**: `ContextError`, `UnitError`, `BuildError`, `SemanticError`, `SemanticErrorKind`, `SemanticErrors`

This fragmentation causes:
1. Inconsistent error handling patterns across crates
2. Difficulty propagating errors between layers
3. Manual `From` implementations for each error type pair
4. No unified way for users to handle "any AngelScript error"
5. `Span` is defined in parser but needed everywhere for error locations
6. Overlapping error types: `FfiRegistryError` and `ModuleError` both handle registration failures

## Solution Overview

Create a unified `AngelScriptError` type in `angelscript-core` that:
1. Acts as a common error type that all other errors can convert into
2. Provides consistent source location tracking via `Span` (moved to core)
3. Uses `thiserror` for ergonomic error handling
4. Preserves the existing specialized error types for fine-grained handling
5. Enables `?` operator usage across crate boundaries
6. Consolidates `FfiRegistryError` and `ModuleError` into a single `RegistrationError`

### Design Approach

**Option A: Single Unified Enum** - One large enum with all error variants
- Pros: Simple, one type to rule them all
- Cons: Large enum, mixed concerns, harder to match specific phases

**Option B: Trait-based Unification** - Common trait, specialized implementations
- Pros: Keeps specialization, trait objects for generic handling
- Cons: Less ergonomic, requires `Box<dyn AngelScriptError>`

**Option C: Layered Wrapper Enum (Recommended)** - Top-level enum wrapping phase-specific errors
- Pros: Preserves specialization, clear phase separation, ergonomic `From` impls
- Cons: Extra wrapper layer

### Chosen Approach: Option C

```rust
// In angelscript-core/src/error.rs
#[derive(Debug, Error)]
pub enum AngelScriptError {
    #[error("lexer error: {0}")]
    Lexer(#[from] LexerError),

    #[error("parse error: {0}")]
    Parse(#[from] ParseError),

    #[error("registration error: {0}")]
    Registration(#[from] RegistrationError),

    #[error("compilation error: {0}")]
    Compilation(#[from] CompilationError),

    #[error("runtime error: {0}")]
    Runtime(#[from] RuntimeError),
}
```

### Consolidated Registration Errors

Merge `FfiRegistryError` and `ModuleError` into a single `RegistrationError`:

```rust
#[derive(Debug, Clone, Error)]
pub enum RegistrationError {
    #[error("type not found: {0}")]
    TypeNotFound(String),

    #[error("duplicate type: {0}")]
    DuplicateType(String),

    #[error("duplicate registration: {name} already registered as {kind}")]
    DuplicateRegistration { name: String, kind: String },

    #[error("duplicate enum value: '{value_name}' in enum '{enum_name}'")]
    DuplicateEnumValue { enum_name: String, value_name: String },

    #[error("invalid declaration: {0}")]
    InvalidDeclaration(String),

    #[error("invalid type: {0}")]
    InvalidType(String),
}
```

This replaces both:
- `FfiRegistryError` (2 variants: `TypeNotFound`, `DuplicateType`)
- `ModuleError` (5 variants: `InvalidDeclaration`, `DuplicateRegistration`, `DuplicateEnumValue`, `TypeNotFound`, `InvalidType`)

## Session-Sized Tasks

| # | Task | Description | Dependencies | Status |
|---|------|-------------|--------------|--------|
| 1 | Move Span to core | Move `Span` from parser to angelscript-core | None | ✅ Complete |
| 2 | Create core error types | Define `AngelScriptError` and phase-specific errors in core | 1 | ✅ Complete |
| 3 | Migrate parser errors | Update parser to use Span and errors from core | 2 | ✅ Complete |
| 4 | Consolidate registration errors | Merge `FfiRegistryError` + `ModuleError` → `RegistrationError` | 2 | Pending |
| 5 | Migrate compiler errors | Update compiler errors to use core types | 2 | Pending |
| 6 | Migrate main crate errors | Update `ContextError`, `UnitError`, `BuildError` | 2-5 | Pending |
| 7 | Update public API | Expose `AngelScriptError` in public API | 6 | Pending |

## Task Details

### Task 1: Move Span to Core

**Files to modify:**
- `crates/angelscript-core/src/span.rs` (new)
- `crates/angelscript-core/src/lib.rs`
- `crates/angelscript-parser/src/lexer/span.rs` (delete or re-export)
- `crates/angelscript-parser/src/lexer/mod.rs`
- `crates/angelscript-parser/Cargo.toml` (add core dependency)

**Changes:**
1. Copy `Span` struct to `angelscript-core/src/span.rs`
2. Export from core: `pub use span::Span;`
3. Make parser depend on core
4. Replace parser's Span with re-export: `pub use angelscript_core::Span;`
5. Update all imports in parser

### Task 2: Create Core Error Types

**Files to modify:**
- `crates/angelscript-core/src/error.rs` (new)
- `crates/angelscript-core/src/lib.rs`
- `crates/angelscript-core/Cargo.toml` (add thiserror)

**New types:**
```rust
use thiserror::Error;
use crate::Span;

// ============================================================================
// Lexer Errors
// ============================================================================

#[derive(Debug, Clone, PartialEq, Error)]
pub enum LexError {
    #[error("unexpected character '{ch}' at {span}")]
    UnexpectedChar { ch: char, span: Span },

    #[error("unterminated string at {span}")]
    UnterminatedString { span: Span },

    #[error("unterminated heredoc at {span}")]
    UnterminatedHeredoc { span: Span },

    #[error("unterminated comment at {span}")]
    UnterminatedComment { span: Span },

    #[error("invalid number at {span}: {detail}")]
    InvalidNumber { span: Span, detail: String },
}

// ============================================================================
// Parse Errors
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseErrorKind {
    // Token-level errors
    ExpectedToken,
    UnexpectedToken,
    UnexpectedEof,

    // Expression errors
    ExpectedExpression,
    ExpectedOperator,
    InvalidExpression,
    ExpectedPrimary,

    // Type errors
    ExpectedType,
    InvalidType,
    ExpectedTemplateArgs,

    // Statement errors
    ExpectedStatement,
    InvalidStatement,
    ExpectedBlock,

    // Declaration errors
    ExpectedDeclaration,
    InvalidDeclaration,
    ExpectedParameters,
    ExpectedClassMember,
    ExpectedInterfaceMethod,

    // Identifier errors
    ExpectedIdentifier,
    DuplicateIdentifier,

    // Scope/namespace errors
    InvalidScope,
    ExpectedNamespace,

    // Control flow errors
    BreakOutsideLoop,
    ContinueOutsideLoop,

    // Syntax errors
    MismatchedDelimiter,
    MissingSemicolon,
    InvalidSyntax,

    // Modifier errors
    InvalidModifier,
    ConflictingModifiers,

    // Other
    InternalError,
    NotImplemented,
}

#[derive(Debug, Clone, PartialEq, Error)]
#[error("parse error at {span}: {message}")]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub span: Span,
    pub message: String,
}

// ============================================================================
// Registration Errors (merged from FfiRegistryError + ModuleError)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Error)]
pub enum RegistrationError {
    #[error("type not found: {0}")]
    TypeNotFound(String),

    #[error("duplicate type: {0}")]
    DuplicateType(String),

    #[error("duplicate registration: {name} already registered as {kind}")]
    DuplicateRegistration { name: String, kind: String },

    #[error("duplicate enum value: '{value_name}' in enum '{enum_name}'")]
    DuplicateEnumValue { enum_name: String, value_name: String },

    #[error("invalid declaration: {0}")]
    InvalidDeclaration(String),

    #[error("invalid type: {0}")]
    InvalidType(String),
}

// ============================================================================
// Compilation Errors
// ============================================================================

#[derive(Debug, Clone, PartialEq, Error)]
pub enum CompilationError {
    #[error("at {span}: unknown type '{name}'")]
    UnknownType { name: String, span: Span },

    #[error("at {span}: unknown function '{name}'")]
    UnknownFunction { name: String, span: Span },

    #[error("at {span}: ambiguous type '{name}': could be {candidates}")]
    AmbiguousType { name: String, candidates: String, span: Span },

    #[error("at {span}: {message}")]
    TypeMismatch { message: String, span: Span },

    #[error("at {span}: {message}")]
    InvalidOperation { message: String, span: Span },

    #[error("at {span}: circular inheritance for '{name}'")]
    CircularInheritance { name: String, span: Span },

    #[error("at {span}: {message}")]
    Other { message: String, span: Span },
}

// ============================================================================
// Runtime Errors
// ============================================================================

#[derive(Debug, Clone, PartialEq, Error)]
pub enum RuntimeError {
    #[error("type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    #[error("null handle cannot be converted to {target_type}")]
    NullHandle { target_type: String },

    #[error("integer overflow: {value} doesn't fit in {target_type}")]
    IntegerOverflow { value: i64, target_type: String },

    #[error("invalid UTF-8 string")]
    InvalidUtf8,

    #[error("stale handle: object at index {index} has been freed")]
    StaleHandle { index: u32 },

    #[error("native function panicked: {message}")]
    NativePanic { message: String },

    #[error("{message}")]
    Other { message: String },
}

// ============================================================================
// Unified Error Type
// ============================================================================

#[derive(Debug, Clone, PartialEq, Error)]
pub enum AngelScriptError {
    #[error(transparent)]
    Lex(#[from] LexError),

    #[error(transparent)]
    Parse(#[from] ParseError),

    #[error(transparent)]
    Registration(#[from] RegistrationError),

    #[error(transparent)]
    Compilation(#[from] CompilationError),

    #[error(transparent)]
    Runtime(#[from] RuntimeError),
}
```

### Task 3: Migrate Parser Errors

**Files to modify:**
- `crates/angelscript-parser/src/lexer/error.rs` (delete)
- `crates/angelscript-parser/src/lexer/mod.rs`
- `crates/angelscript-parser/src/ast/error.rs` (delete)
- `crates/angelscript-parser/src/ast/mod.rs`
- `crates/angelscript-parser/src/lib.rs`

**Approach:**
1. Parser already depends on angelscript-core
2. Re-export error types from core: `pub use angelscript_core::{LexError, ParseError, ParseErrorKind};`
3. Delete `lexer/error.rs` and `ast/error.rs`
4. Update all usages to use the core types
5. Keep `ParseErrors` collection struct (or move to core)

### Task 4: Consolidate Registration Errors

**Files to modify:**
- `crates/angelscript-ffi/src/registry/ffi_registry.rs` (delete `FfiRegistryError`)
- `crates/angelscript-ffi/src/lib.rs`
- `crates/angelscript-module/src/module.rs` (delete `ModuleError`)
- `crates/angelscript-module/src/lib.rs`
- All builders that return `ModuleError`

**Approach:**
1. Both crates depend on angelscript-core
2. Replace `FfiRegistryError` with `RegistrationError` from core
3. Replace `ModuleError` with `RegistrationError` from core
4. Update `FfiRegistryBuilder::build()` to return `Result<_, Vec<RegistrationError>>`
5. Update all builder methods to return `Result<_, RegistrationError>`

### Task 5: Migrate Compiler Errors

**Files to modify:**
- `crates/angelscript-compiler/src/passes/registration.rs`
- `crates/angelscript-compiler/src/context.rs`

**Approach:**
1. Replace local `RegistrationError` with `CompilationError` from core
2. Replace `ResolutionError` with `CompilationError` variants
3. Update pass return types

### Task 6: Migrate Main Crate Errors

**Files to modify:**
- `src/context.rs`
- `src/unit.rs`
- `src/semantic/error.rs`

**Approach:**
1. Replace `ContextError` variants with `AngelScriptError` where appropriate
2. Update `BuildError` to wrap `AngelScriptError` or `Vec<AngelScriptError>`
3. Keep `SemanticError` temporarily (legacy, will be replaced by new compiler)
4. Simplify with `?` operator

### Task 7: Update Public API

**Files to modify:**
- `src/lib.rs`

**Approach:**
1. Export from main crate:
   ```rust
   pub use angelscript_core::{
       AngelScriptError, LexError, ParseError, ParseErrorKind,
       RegistrationError, CompilationError, RuntimeError, Span,
   };
   ```
2. Document error handling in module docs

## Testing Strategy

1. **Unit Tests**: Each error type has tests for:
   - Display formatting
   - `From` conversions between error types
   - Clone/Debug/PartialEq traits

2. **Integration Tests**: Verify error propagation through:
   - Lexer → Parser → Compiler → Build pipeline
   - Registration errors from FFI and Module builders
   - Runtime errors during execution

3. **Regression Tests**: Ensure existing error messages aren't degraded
   - Existing tests should pass unchanged (same error messages)

## Risks & Considerations

### Breaking Changes
- `FfiRegistryError` removed → replaced by `RegistrationError`
- `ModuleError` removed → replaced by `RegistrationError`
- Users matching on these types need updates
- Mitigation: Type aliases for deprecation period

### Performance
- Wrapping errors adds minimal overhead (enum discriminant)
- `Span` is already `Copy` - no performance concern
- All error types derive `Clone` for flexibility

### Crate Dependencies
- angelscript-core gains `thiserror` dependency (small, no_std compatible)
- Parser already depends on core (no new edge)
- FFI and Module crates already depend on core

## Decisions Made

1. **Span in core** - Move to core for cleaner dependency graph
2. **RuntimeError included** - Consolidates `ConversionError` and `NativeError`
3. **RegistrationError consolidation** - Merge `FfiRegistryError` + `ModuleError`
4. **Keep ParseErrors collection** - Useful for accumulating multiple parse errors
