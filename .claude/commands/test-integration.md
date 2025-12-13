---
allowed-tools: Bash(cargo nextest:*)
argument-hint: [test_harness|module_tests]
description: Run integration tests
---

Run integration tests from the tests/ directory using nextest.

Available test files:
- `test_harness` - Parser integration tests using test_scripts/*.as
- `module_tests` - Module/runtime integration tests

$ARGUMENTS provided: `cargo nextest run --test $ARGUMENTS`
No arguments (run all): `cargo nextest run --test '*'`
