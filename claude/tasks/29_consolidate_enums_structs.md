# Task 29: Consolidate Enums/Structs Between Parser and FFI

## Problem Summary

The parser and core crates have duplicate or near-duplicate type definitions:
- **BinaryOp**: Identical in both (parser: `ast/ops.rs`, core: `ops.rs`)
- **UnaryOp**: Identical in both (parser: `ast/ops.rs`, core: `ops.rs`)
- **PrimitiveType**: Different variant naming (parser uses `Int/UInt`, core uses `Int32/Uint32`)
- **Visibility**: Identical in both (parser: `ast/node.rs`, core: `type_def.rs`)

This duplication leads to:
1. Maintenance burden - changes must be made in multiple places
2. Potential drift - types could evolve inconsistently
3. Conversion overhead - types must be converted between parser and compiler

## Solution Overview

Consolidate duplicated types into `angelscript-core`, then have the parser import from core. The parser will depend on core for shared types.

## Consolidation Analysis

| Type | Parser | Core | Action |
|------|--------|------|--------|
| BinaryOp | `ast/ops.rs` | `ops.rs` | **Remove from parser**, keep parser-specific methods as extension trait |
| UnaryOp | `ast/ops.rs` | `ops.rs` | **Remove from parser**, keep parser-specific methods as extension trait |
| PostfixOp | `ast/ops.rs` | N/A | **Move to core**, keep parser-specific methods as extension trait |
| AssignOp | `ast/ops.rs` | N/A | **Move to core**, keep parser-specific methods as extension trait |
| PrimitiveKind | `ast/types.rs` | `type_def.rs` | **Rename** core's `PrimitiveType` to `PrimitiveKind`, unify naming |
| Visibility | `ast/node.rs` | `type_def.rs` | **Remove from parser**, use core |
| RefKind | `ast/node.rs` | N/A | **Keep in parser** - different semantics from RefModifier |

**Naming Convention**: Use `Kind` suffix for enum types (e.g., `PrimitiveKind`, `RefKind`, `TypeKind`).

## Session-Sized Tasks

| # | Task | Description | Dependencies | Status |
|---|------|-------------|--------------|--------|
| 1 | Add parser dependency on core | Update `crates/angelscript-parser/Cargo.toml` to depend on `angelscript-core` | None | Pending |
| 2 | Rename PrimitiveType to PrimitiveKind | Rename in core, update all references in core and dependent crates | 1 | Pending |
| 3 | Consolidate BinaryOp | Remove from parser, import from core, add parser extension trait for `from_token`/`binding_power` | 1 | Pending |
| 4 | Consolidate UnaryOp | Remove from parser, import from core, add parser extension trait for `from_token`/`binding_power` | 3 | Pending |
| 5 | Move PostfixOp to core | Add to core's ops.rs, remove from parser, add extension trait | 3 | Pending |
| 6 | Move AssignOp to core | Add to core's ops.rs, remove from parser, add extension trait | 5 | Pending |
| 7 | Consolidate Visibility | Remove from parser, import from core | 1 | Pending |
| 8 | Unify PrimitiveKind variants | Update parser to use core's `Int32/Uint32` naming, remove parser's PrimitiveType | 2 | Pending |
| 9 | Test and verify | Run all tests, ensure no regressions | 1-8 | Pending |

## Task Details

### Task 1: Add Parser Dependency on Core

**Files:**
- `crates/angelscript-parser/Cargo.toml`

**Changes:**
- Add `angelscript-core = { path = "../angelscript-core" }` to dependencies

### Task 2: Rename PrimitiveType to PrimitiveKind

**Files:**
- `crates/angelscript-core/src/type_def.rs`
- `crates/angelscript-core/src/lib.rs`
- All files in `crates/angelscript-ffi/` that reference `PrimitiveType`
- All files in `crates/angelscript-compiler/` that reference `PrimitiveType`

**Changes:**
1. Rename `PrimitiveType` enum to `PrimitiveKind` in core
2. Update all imports and usages across crates
3. Keep variant names as-is (`Int32`, `Uint32`, etc.)

### Task 3: Consolidate BinaryOp

**Files:**
- `crates/angelscript-parser/src/ast/ops.rs`
- `crates/angelscript-parser/src/ast/mod.rs`

**Changes:**
1. Remove `BinaryOp` enum definition from parser
2. Add `pub use angelscript_core::BinaryOp;` to re-export
3. Create extension trait `BinaryOpExt` in parser for parser-specific methods:
   - `from_token(TokenKind) -> Option<Self>`
   - `binding_power(&self) -> (u8, u8)`
4. Update all usages to use the trait

### Task 4: Consolidate UnaryOp

**Files:**
- `crates/angelscript-parser/src/ast/ops.rs`

**Changes:**
1. Remove `UnaryOp` enum definition from parser
2. Add `pub use angelscript_core::UnaryOp;` to re-export
3. Create extension trait `UnaryOpExt` for:
   - `from_token(TokenKind) -> Option<Self>`
   - `binding_power() -> u8`
4. Update all usages

### Task 5: Move PostfixOp to Core

**Files:**
- `crates/angelscript-core/src/ops.rs` - add PostfixOp enum
- `crates/angelscript-core/src/lib.rs` - export PostfixOp
- `crates/angelscript-parser/src/ast/ops.rs` - remove enum, add extension trait

**Changes:**
1. Add `PostfixOp` enum to core's `ops.rs`:
   - `PostInc` - post-increment (`++`)
   - `PostDec` - post-decrement (`--`)
2. Add `Display` impl in core
3. Remove enum from parser, import from core
4. Create extension trait `PostfixOpExt` for:
   - `from_token(TokenKind) -> Option<Self>`
   - `binding_power() -> u8`

### Task 6: Move AssignOp to Core

**Files:**
- `crates/angelscript-core/src/ops.rs` - add AssignOp enum
- `crates/angelscript-core/src/lib.rs` - export AssignOp
- `crates/angelscript-parser/src/ast/ops.rs` - remove enum, add extension trait

**Changes:**
1. Add `AssignOp` enum to core's `ops.rs`:
   - `Assign`, `AddAssign`, `SubAssign`, `MulAssign`, `DivAssign`, `ModAssign`, `PowAssign`
   - `AndAssign`, `OrAssign`, `XorAssign`, `ShlAssign`, `ShrAssign`, `UshrAssign`
2. Add `Display` impl in core
3. Remove enum from parser, import from core
4. Create extension trait `AssignOpExt` for:
   - `from_token(TokenKind) -> Option<Self>`
   - `binding_power() -> (u8, u8)`
   - `is_simple(&self) -> bool`

### Task 7: Consolidate Visibility

**Files:**
- `crates/angelscript-parser/src/ast/node.rs`
- `crates/angelscript-parser/src/ast/mod.rs`

**Changes:**
1. Remove `Visibility` enum from parser
2. Add `pub use angelscript_core::Visibility;`
3. Parser's `Visibility::default()` method - core already uses `#[default]` derive, compatible

### Task 8: Unify PrimitiveKind Variants

**Files:**
- `crates/angelscript-parser/src/ast/types.rs`
- `crates/angelscript-parser/src/ast/type_parser.rs`
- Other parser files using `PrimitiveType`

**Changes:**
1. Remove `PrimitiveType` enum from parser
2. Import `PrimitiveKind` from core
3. Update variant references: `Int` -> `Int32`, `UInt` -> `Uint32`
4. Create extension trait `PrimitiveKindExt` for parser-specific methods:
   - `size_bytes(&self) -> usize`
   - `is_integer(&self) -> bool`
   - `is_float(&self) -> bool`
   - `is_signed(&self) -> bool`
   - `is_unsigned(&self) -> bool`

## Testing Strategy

1. Run `cargo test -p angelscript-core` after renaming PrimitiveType
2. Run `cargo test -p angelscript-parser` after each parser task
3. Run full test suite `cargo test --lib` after all tasks
4. Verify build with `cargo build`

## Risks & Considerations

1. **Renaming in core**: Renaming `PrimitiveType` to `PrimitiveKind` affects FFI/compiler crates - must update all references
2. **Extension traits**: Using extension traits adds some complexity but maintains clean separation between core types and parser-specific behavior
3. **Circular dependencies**: Parser -> core dependency is safe since core has no dependencies

## Critical Files to Modify

**Core crate:**
- `crates/angelscript-core/src/type_def.rs` - rename PrimitiveType to PrimitiveKind
- `crates/angelscript-core/src/ops.rs` - add PostfixOp and AssignOp enums
- `crates/angelscript-core/src/lib.rs` - update exports

**Parser crate:**
- `crates/angelscript-parser/Cargo.toml` - add core dependency
- `crates/angelscript-parser/src/ast/ops.rs` - consolidate all operator types, add extension traits
- `crates/angelscript-parser/src/ast/node.rs` - consolidate Visibility
- `crates/angelscript-parser/src/ast/types.rs` - use PrimitiveKind from core
- `crates/angelscript-parser/src/ast/mod.rs` - update re-exports

**Dependent crates (for PrimitiveType rename):**
- `crates/angelscript-ffi/src/**/*.rs`
- `crates/angelscript-compiler/src/**/*.rs`
