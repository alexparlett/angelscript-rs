---
allowed-tools: Bash(cargo build:*)
argument-hint: [--release]
description: Build the library
---

Build the angelscript library.

$ARGUMENTS contains "release": `cargo build --lib --release`
Otherwise: `cargo build --lib`
