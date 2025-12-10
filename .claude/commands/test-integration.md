---
allowed-tools: Bash(cargo nextest run:*)
argument-hint: [test_harness|module_tests]
description: Run integration tests
---

Run integration tests from the tests/ directory with TDD Guard integration.

Available test files:
- `test_harness` - Parser integration tests using test_scripts/*.as
- `module_tests` - Module/runtime integration tests

$ARGUMENTS provided: `cargo nextest run --test $ARGUMENTS 2>&1 | tdd-guard-rust --project-root /Users/alexparlett/Development/angelscript-rust --passthrough`
No arguments (run all): `cargo nextest run --test test_harness 2>&1 | tdd-guard-rust --project-root /Users/alexparlett/Development/angelscript-rust --passthrough && cargo nextest run --test module_tests 2>&1 | tdd-guard-rust --project-root /Users/alexparlett/Development/angelscript-rust --passthrough`
