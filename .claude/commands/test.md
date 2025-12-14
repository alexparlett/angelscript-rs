---
allowed-tools: Bash(cargo nextest:*)
argument-hint: [filter]
description: Run unit tests with optional filter
---

Run cargo library tests using nextest across the entire workspace. If a filter is provided, run only matching tests.

$ARGUMENTS provided: `cargo nextest run --workspace -E 'test($ARGUMENTS)'`
No arguments: `cargo nextest run --workspace`
