# Task 32b: AST String Literal Byte Storage

## Overview

Change the AST to store string literals as raw bytes (`Vec<u8>`) instead of `String`. This aligns with the StringFactory design (Task 32) and enables proper support for non-UTF8 escape sequences.

## Goals

1. Change `LiteralKind::String(String)` to `LiteralKind::String(Vec<u8>)`
2. Update the lexer to produce raw bytes (with escape sequence processing)
3. Update the parser to pass through raw bytes
4. Update tests

## Dependencies

- Task 32: String Factory Configuration (completed)

## Background

The StringFactory trait takes `&[u8]` to support:
- Non-UTF8 escape sequences (`\xFF`, `\x00`, etc.)
- Custom string encodings (ASCII, Latin-1, etc.)
- OsString and other non-UTF8 string types

Currently the AST stores `String`, which:
- Forces UTF-8 encoding at parse time
- Loses the ability to represent arbitrary byte sequences
- Contradicts the StringFactory design

## Detailed Implementation

### Phase 1: LiteralKind Change

**File:** `crates/angelscript-parser/src/ast/expr.rs`

```rust
// Before
pub enum LiteralKind {
    Int(i64),
    Float(f32),
    Double(f64),
    Bool(bool),
    String(String),  // UTF-8 only
    Null,
}

// After
pub enum LiteralKind {
    Int(i64),
    Float(f32),
    Double(f64),
    Bool(bool),
    String(Vec<u8>),  // Raw bytes - factory interprets encoding
    Null,
}
```

### Phase 2: Lexer Updates

**File:** `crates/angelscript-parser/src/lexer/mod.rs`

The lexer already processes escape sequences. Update to:
1. Store raw bytes instead of decoded UTF-8
2. Support `\xNN` escape sequences that produce arbitrary bytes
3. Return `Vec<u8>` for string token values

Key escape sequences to support:
- `\n`, `\r`, `\t`, `\\`, `\"`, `\'` - standard escapes
- `\xNN` - arbitrary byte value (00-FF)
- `\0` - null byte

### Phase 3: Parser Updates

**File:** `crates/angelscript-parser/src/ast/expr_parser.rs`

Update string literal parsing to:
1. Extract raw bytes from lexer token
2. Create `LiteralKind::String(bytes)`

```rust
// Before (approx line 184-190)
TokenKind::StringLiteral | TokenKind::HeredocLiteral => {
    self.advance();
    let content = token.lexeme.trim_matches('"');
    // ... creates String
}

// After
TokenKind::StringLiteral | TokenKind::HeredocLiteral => {
    self.advance();
    let bytes = self.process_string_literal(token)?;
    Expr::Literal(LiteralExpr {
        kind: LiteralKind::String(bytes),
        span,
    })
}
```

### Phase 4: Test Updates

Update all tests that use string literals to work with `Vec<u8>`:

```rust
// Before
assert_eq!(lit.kind, LiteralKind::String("hello".to_string()));

// After
assert_eq!(lit.kind, LiteralKind::String(b"hello".to_vec()));
```

## Escape Sequence Processing

The lexer should process escape sequences and produce raw bytes:

| Escape | Bytes |
|--------|-------|
| `\n` | `[0x0A]` |
| `\r` | `[0x0D]` |
| `\t` | `[0x09]` |
| `\\` | `[0x5C]` |
| `\"` | `[0x22]` |
| `\'` | `[0x27]` |
| `\0` | `[0x00]` |
| `\xNN` | `[0xNN]` |

For invalid escape sequences, emit a parse error.

## Files to Modify

1. `crates/angelscript-parser/src/ast/expr.rs` - LiteralKind enum
2. `crates/angelscript-parser/src/lexer/mod.rs` - String lexing
3. `crates/angelscript-parser/src/ast/expr_parser.rs` - String parsing
4. Various test files

## Testing

```rust
#[test]
fn parse_string_literal_bytes() {
    let (script, _) = parse("string s = \"hello\";");
    // Verify LiteralKind::String contains b"hello"
}

#[test]
fn parse_string_escape_sequences() {
    let (script, _) = parse(r#"string s = "a\nb\tc";"#);
    // Verify bytes are [0x61, 0x0A, 0x62, 0x09, 0x63]
}

#[test]
fn parse_string_hex_escape() {
    let (script, _) = parse(r#"string s = "\xFF\x00";"#);
    // Verify bytes are [0xFF, 0x00]
}

#[test]
fn parse_heredoc_bytes() {
    let (script, _) = parse(r#"string s = """raw""";"#);
    // Verify raw bytes preserved
}
```

## Acceptance Criteria

- [x] `LiteralKind::String` stores `Vec<u8>`
- [x] Parser produces raw bytes with escape processing
- [x] `\xNN` escape sequences work correctly
- [x] Heredoc strings preserve raw bytes
- [x] All existing tests pass (updated for new type)
- [x] New tests for byte escape sequences

## Notes

- This is a breaking change to the AST structure
- Debug output will show bytes instead of readable strings
- Consider adding a helper to display string literals for error messages
