---
allowed-tools: Bash(cargo bench:*)
argument-hint: [group|name]
description: Run benchmarks with optional group filter
---

Run Criterion benchmarks.

Available benchmark groups:
- `unit/file_sizes` - File size scaling (5-5000 lines)
- `unit/features` - Feature-specific (functions, classes, expressions)
- `unit/real_world` - Real-world scripts (game logic, utilities)
- `unit/complexity` - Complexity-based tests

Examples:
- Run all: `cargo bench`
- Run group: `cargo bench -- "unit/file_sizes"`
- Run single: `cargo bench -- "stress_5000"`
- With profiling: `cargo bench --features profile-with-puffin`

$ARGUMENTS provided: `cargo bench -- "$ARGUMENTS"`
No arguments: `cargo bench`
