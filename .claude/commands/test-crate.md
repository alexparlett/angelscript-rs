---
allowed-tools: Bash(cargo nextest run:*)
argument-hint: <crate-name>
description: Run tests for a specific crate
---

Run tests for a specific workspace crate with TDD Guard integration.

Available crates:
- `angelscript-core` - Shared types
- `angelscript-parser` - Lexer + AST + Parser
- `angelscript-ffi` - FFI registry
- `angelscript-compiler` - 2-pass compiler

Run: `cargo nextest run -p $ARGUMENTS 2>&1 | tdd-guard-rust --project-root /Users/alexparlett/Development/angelscript-rust --passthrough`
