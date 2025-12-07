---
allowed-tools: Bash(cargo test:*)
argument-hint: [filter]
description: Run unit tests with optional filter
---

Run cargo library tests. If a filter is provided, run only matching tests.

$ARGUMENTS provided: `cargo test --lib $ARGUMENTS`
No arguments: `cargo test --lib`
