---
allowed-tools: Bash(cargo clippy:*)
argument-hint: [fix]
description: Run clippy linter (with optional auto-fix)
---

Run cargo clippy for lint warnings and suggestions across the entire workspace.

$ARGUMENTS contains "fix": `cargo clippy --workspace --all-targets --fix --allow-dirty`
Otherwise: `cargo clippy --workspace --all-targets`
