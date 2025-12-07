---
allowed-tools: Bash(cargo test:*)
argument-hint: <crate-name>
description: Run tests for a specific crate
---

Run tests for a specific workspace crate.

Available crates:
- `angelscript-core` - Shared types
- `angelscript-parser` - Lexer + AST + Parser
- `angelscript-ffi` - FFI registry
- `angelscript-compiler` - 2-pass compiler

Run: `cargo test -p $ARGUMENTS`
