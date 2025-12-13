---
allowed-tools: Bash(cargo check:*)
description: Quick compile check without building
---

Run cargo check to verify code compiles without producing binaries. Faster than a full build.

Run: `cargo check --workspace --all-targets`
