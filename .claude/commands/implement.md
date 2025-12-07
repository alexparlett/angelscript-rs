---
allowed-tools: Bash(cargo:*), Bash(git:*), Glob, Grep, Read, Edit, Write
argument-hint: <task-file-or-number>
description: Implement a feature from its design document
---

# Feature Implementation

You are implementing a feature from its design document.

## Context

1. Load the design document:
   - If a number is provided (e.g., `26`), read `claude/tasks/26_*.md`
   - If a path is provided, read that file directly

2. Check current prompt for context:
   `claude/prompt.md`

3. Check git status:
   `git status --short`

## Task Reference

Design document: `$ARGUMENTS`

Read the design document and identify:
- Which sub-task to work on next (first `Pending` task)
- Files that need to be modified
- Dependencies on previous tasks

## Implementation Process

For each sub-task:

1. **Mark task as in-progress** in the design document
2. **Confirm approach** with user before coding (per CLAUDE.md rules)
3. **Implement the changes** following project patterns:
   - Use `TypeHash` for type identity (not `format!()`)
   - Prefer `Option` returns over panics
   - Use `thiserror` for error types
   - Follow naming conventions (`get_*`, `lookup_*`, `find_*`)
4. **Run tests**: `/test`
5. **Run linter**: `/clippy`
6. **Mark task complete** in the design document
7. **Commit the work**: `git add && git commit`

## Quality Checks

Before marking a task complete:

- [ ] Tests pass: `cargo test --lib`
- [ ] Build succeeds: `cargo build --lib`
- [ ] No clippy warnings: `cargo clippy --all-targets`
- [ ] Code follows project patterns (see CLAUDE.md)
- [ ] No `Rc<RefCell<>>` in public APIs
- [ ] Error types use `thiserror`

## After Implementation

1. Update the task status in the design document
2. Update `claude/prompt.md` with progress
3. Create a descriptive commit (DO NOT push unless asked)

## Important Rules

- **Confirm before coding** - Always explain changes and wait for approval
- **One sub-task at a time** - Don't batch multiple sub-tasks
- **Commit after each sub-task** - Preserve work incrementally
- **Never use git checkout/restore** - If you need to undo, use Edit tool
