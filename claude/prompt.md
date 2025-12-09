# Current Task: AST String Literal Byte Storage (Task 32b)

**Status:** Complete
**Date:** 2025-12-09
**Branch:** 032b-ast-string-bytes

---

## Task 32b: AST String Literal Byte Storage

Changed `LiteralKind::String` from `String` to `Vec<u8>` to support non-UTF8
escape sequences and proper StringFactory integration.

### Implementation Summary

| Component | Location | Purpose |
|-----------|----------|---------|
| `LiteralKind::String(Vec<u8>)` | angelscript-parser/src/ast/expr.rs | Store raw bytes instead of String |
| `process_string_bytes()` | angelscript-parser/src/ast/expr_parser.rs | Parse escape sequences into bytes |
| `InvalidEscapeSequence` | angelscript-core/src/error.rs | New error variant for bad escapes |

### Escape Sequences Supported

| Escape | Byte |
|--------|------|
| `\n` | 0x0A (newline) |
| `\r` | 0x0D (carriage return) |
| `\t` | 0x09 (tab) |
| `\\` | 0x5C (backslash) |
| `\"` | 0x22 (double quote) |
| `\'` | 0x27 (single quote) |
| `\0` | 0x00 (null byte) |
| `\xNN` | 0xNN (hex byte) |

### Key Files

- `crates/angelscript-parser/src/ast/expr.rs` - LiteralKind enum change
- `crates/angelscript-parser/src/ast/expr_parser.rs` - Escape sequence processing
- `crates/angelscript-core/src/error.rs` - InvalidEscapeSequence error
- `claude/tasks/32b_ast_string_bytes.md` - Full task spec

---

## Complete

Task 32b is complete. The AST now stores string literals as raw bytes:
- `LiteralKind::String(Vec<u8>)` instead of `LiteralKind::String(String)`
- Full escape sequence processing (\n, \r, \t, \\, \", \', \0, \xNN)
- Heredoc strings preserve raw bytes (no escape processing)
- 8 new tests for escape sequence handling
- All 601 parser tests pass

## Next Steps

Task 33: Compilation Context - wraps TypeRegistry with compilation state
