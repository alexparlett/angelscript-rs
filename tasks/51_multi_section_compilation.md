# Task 51: Multi-Section Script Compilation

## Problem Summary

AngelScript supports compiling multiple script sections (named source strings) into a single module via `AddScriptSection(name, code)` followed by `Build()`. Currently, the Rust implementation has a blocking limitation at `src/unit.rs:288-291` that returns `BuildError::MultiFileNotSupported` if more than one source is added.

The infrastructure exists (multiple sources can be stored, parsed, change-tracked) but the compilation pipeline doesn't wire them together.

## Solution Overview

Enable multi-section compilation by:
1. Adding section name (`&'ast str`) to `Span` - lifetime-bound to arena
2. Parser returns `&[Item]` directly (no `Script` wrapper), takes section name
3. Errors copy section name to owned `String` when created (errors outlive arena)
4. Compiler stays section-agnostic - just sees items with spans
5. Remove the single-file limitation in Unit

## Key Design Decisions

1. **Section in Span as `&'ast str`**: Span lives only during compilation (in arena), so lifetime-bound string is fine. Zero-cost during compilation.

2. **Parser returns items, not Script**: `Script` wrapper isn't needed - parser returns `&'ast [Item<'ast>]` directly. Unit concatenates items from all sections.

3. **Errors own section name**: When creating an error from a span, copy `span.section.to_string()`. Errors are self-contained, no fixup needed.

4. **Compiler is section-agnostic**: Compiler just sees items with spans. It doesn't know or care about sections - that's Unit's concern.

## Session-Sized Tasks

| # | Task | Description | Dependencies | Status |
|---|------|-------------|--------------|--------|
| 1 | Span with Section | Add `section: &'ast str` to Span, update all usages | None | Pending |
| 2 | Parser Returns Items | Remove Script wrapper, parser takes section name | 1 | Pending |
| 3 | Error Section Support | Errors copy section name from span | 1 | Pending |
| 4 | Unit Multi-Section | Remove limitation, concat items, update build | 2,3 | Pending |
| 5 | Integration Tests | Cross-section tests, error message tests | 4 | Pending |

---

## Task Details

### Task 1: Span with Section

**Files:** `crates/angelscript-core/src/span.rs`, all files using Span

Add lifetime and section to Span:

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span<'ast> {
    pub section: &'ast str,  // arena-allocated section name
    pub line: u32,
    pub col: u32,
    pub len: u32,
}

impl<'ast> Span<'ast> {
    pub fn new(section: &'ast str, line: u32, col: u32, len: u32) -> Self;
}

impl Default for Span<'static> {
    fn default() -> Self {
        Span { section: "", line: 1, col: 1, len: 0 }
    }
}
```

Update all AST nodes that contain Span to have `'ast` lifetime (they already do for other reasons).

---

### Task 2: Parser Returns Items

**Files:** `crates/angelscript-parser/src/ast/mod.rs`, `crates/angelscript-parser/src/ast/parser.rs`

Remove `Script` struct. Update parser API:

```rust
impl<'ast> Parser<'ast> {
    /// Parse a section, returning items directly
    pub fn parse_section(
        source: &str,
        section: &str,  // will be arena-allocated
        arena: &'ast Bump,
    ) -> Result<&'ast [Item<'ast>], ParseErrors>;

    /// Lenient version for building
    pub fn parse_section_lenient(
        source: &str,
        section: &str,
        arena: &'ast Bump,
    ) -> (&'ast [Item<'ast>], Vec<ParseError>);
}
```

Lexer stamps section name on all tokens/spans.

---

### Task 3: Error Section Support

**Files:** `crates/angelscript-core/src/error.rs`, `crates/angelscript-parser/src/error.rs`

Errors own the section name:

```rust
pub struct ParseError {
    pub section: String,  // owned copy from span
    pub line: u32,
    pub col: u32,
    pub message: String,
}

impl ParseError {
    pub fn new(span: Span<'_>, message: impl Into<String>) -> Self {
        Self {
            section: span.section.to_string(),
            line: span.line,
            col: span.col,
            message: message.into(),
        }
    }
}
```

Same pattern for `CompilationError`.

---

### Task 4: Unit Multi-Section

**Files:** `src/unit.rs`

Remove the limitation and update build:

```rust
pub fn build(&mut self) -> Result<(), BuildError> {
    // Parse all sections, collect items
    let mut all_items: Vec<&Item> = Vec::new();
    let mut all_errors: Vec<ParseError> = Vec::new();

    for (section_name, source) in &self.sources {
        let section_str = self.arena.alloc_str(section_name);
        let (items, errors) = Parser::parse_section_lenient(source, section_str, &self.arena);
        all_items.extend(items);
        all_errors.extend(errors);
    }

    if !all_errors.is_empty() {
        return Err(BuildError::ParseErrors(all_errors));
    }

    // Compile all items together
    let result = Compiler::compile(&all_items, ...);
    // ...
}
```

---

### Task 5: Integration Tests

**Files:** `tests/module_tests.rs`

Test cases:
- Two sections with cross-references
- Error messages show correct section name
- Order independence
- Duplicate section name handling

---

## Testing Strategy

- Unit tests for Span with section
- Parser tests with section names
- Error formatting tests
- Integration tests for multi-section builds

## Risks & Considerations

1. **Lifetime propagation**: Adding `'ast` to Span may require updating many type signatures. AST nodes already have `'ast` so impact should be limited.

2. **Arena lifetime**: All parsed sections share one arena. Ensure arena lives long enough for compilation.

3. **Backwards compatibility**: Existing tests using `Span::new(line, col, len)` need updating to include section.
