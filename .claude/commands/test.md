---
allowed-tools: Bash(cargo nextest run:*)
argument-hint: [filter]
description: Run unit tests with optional filter
---

Run cargo library tests with TDD Guard integration. If a filter is provided, run only matching tests.

$ARGUMENTS provided: `cargo nextest run --lib $ARGUMENTS 2>&1 | tdd-guard-rust --project-root /Users/alexparlett/Development/angelscript-rust --passthrough`
No arguments: `cargo nextest run --lib 2>&1 | tdd-guard-rust --project-root /Users/alexparlett/Development/angelscript-rust --passthrough`
