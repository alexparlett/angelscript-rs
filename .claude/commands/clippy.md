---
allowed-tools: Bash(cargo clippy:*)
argument-hint: [fix]
description: Run clippy linter (with optional auto-fix)
---

Run cargo clippy for lint warnings and suggestions.

$ARGUMENTS contains "fix": `cargo clippy --all-targets --fix --allow-dirty`
Otherwise: `cargo clippy --all-targets`
