---
allowed-tools: Bash(cargo test:*)
argument-hint: [test_harness|module_tests]
description: Run integration tests
---

Run integration tests from the tests/ directory.

Available test files:
- `test_harness` - Parser integration tests using test_scripts/*.as
- `module_tests` - Module/runtime integration tests

$ARGUMENTS provided: `cargo test --test $ARGUMENTS`
No arguments (run all): `cargo test --test test_harness && cargo test --test module_tests`
