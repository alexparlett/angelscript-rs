---
allowed-tools: Bash(git:*), Bash(cargo:*), Glob, Grep, Read
argument-hint: [task-number-or-branch]
description: Review implementation against design and check code quality
---

# Code Review

You are reviewing an implementation for quality and adherence to design.

## Context

1. If a task number is provided, load the design document:
   `claude/tasks/$ARGUMENTS_*.md` or `claude/tasks/$ARGUMENTS`

2. Check what has changed:
   `git diff --stat`
   `git log --oneline -10`

3. Read current prompt:
   `claude/prompt.md`

## Review Process

### 1. Design Adherence

Compare the implementation against the design document:
- [ ] All planned tasks completed?
- [ ] Implementation matches the described approach?
- [ ] Any deviations documented and justified?

### 2. Code Quality

Check against project standards (from CLAUDE.md):

**Patterns - Should Use:**
- [ ] `TypeHash` for type identity (not `format!()`)
- [ ] `Option`/`Result` returns (not panics)
- [ ] `thiserror` for error types
- [ ] `FxHashMap` for hot paths
- [ ] Iterators over index loops
- [ ] Clear ownership (no `Rc<RefCell<>>` in public API)

**Anti-Patterns - Should NOT Have:**
- [ ] No `format!()` for type identity
- [ ] No `clone()` on Copy types
- [ ] No redundant maps for same data
- [ ] No panics on lookup failures

**Naming Conventions:**
- [ ] `*Pass` for compiler passes
- [ ] `*Output` for pass outputs
- [ ] `get_*` for hash lookups
- [ ] `lookup_*` for name lookups
- [ ] `find_*` for searches
- [ ] `*Builder` for builders

### 3. Testing

Run the test suite:
```
cargo test --lib
cargo test --test test_harness
cargo test --test module_tests
```

Check coverage of new code:
- [ ] Unit tests for new functions?
- [ ] Edge cases covered?
- [ ] Error paths tested?

### 4. Performance

If performance-sensitive code:
```
cargo bench -- "[relevant-group]"
```

Check for:
- [ ] No unnecessary allocations in hot paths
- [ ] No `format!()` in hot paths
- [ ] Copy types not being cloned

### 5. Documentation

- [ ] Public APIs documented?
- [ ] Complex logic has comments?
- [ ] Design decisions logged in `claude/decisions.md`?

## Review Output

Provide a summary:

```markdown
## Review Summary

**Task:** [Task number and name]
**Status:** [Approved / Changes Requested]

### What's Good
- ...

### Issues Found
- [ ] Issue 1: [description] - [file:line]
- [ ] Issue 2: [description] - [file:line]

### Suggestions (Optional)
- ...

### Test Results
- Unit tests: [pass/fail]
- Integration tests: [pass/fail]
- Benchmarks: [if applicable]
```

## After Review

If approved:
1. Ensure all commits are in place
2. Update `claude/prompt.md` with completion status
3. Mark task as complete in design document

If changes requested:
1. List specific issues to address
2. Use `/implement` to make fixes
3. Re-run `/review` after fixes
