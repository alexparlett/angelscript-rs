---
allowed-tools: Bash(cargo build:*)
argument-hint: [--release]
description: Build the library
---

Build the entire angelscript workspace.

$ARGUMENTS contains "release": `cargo build --workspace --release`
Otherwise: `cargo build --workspace`
