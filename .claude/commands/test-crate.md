---
allowed-tools: Bash(cargo nextest:*)
argument-hint: <crate-name>
description: Run tests for a specific crate
---

Run tests for a specific workspace crate using nextest.

Available crates:
- `angelscript-core` - Shared types
- `angelscript-parser` - Lexer + AST + Parser
- `angelscript-registry` - Type registry
- `angelscript-compiler` - 2-pass compiler
- `angelscript-macros` - Procedural macros
- `angelscript-modules` - Standard library

Run: `cargo nextest run -p $ARGUMENTS`
