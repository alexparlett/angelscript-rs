---
allowed-tools: Bash(git:*), Bash(find:*), Glob, Grep, Read
argument-hint: <feature-description>
description: Design a feature - analyze codebase and create implementation plan
---

# Feature Design

You are designing a new feature for the AngelScript-Rust project.

## Context

First, understand the current state:

1. Check existing tasks to determine the next task number:
   `find claude/tasks -name "*.md" | sort -V | tail -5`

2. Read the current prompt for active work:
   `claude/prompt.md`

3. Check git status for any in-progress changes:
   `git status --short`

## Your Task

Design the following feature: **$ARGUMENTS**

## Design Process

1. **Analyze the codebase** - Search for relevant code, understand existing patterns
2. **Identify affected components** - Which crates, modules, files need changes
3. **Break into session-sized tasks** - Each task completable in one session without context overflow
4. **Consider edge cases** - Error handling, backwards compatibility, performance
5. **Plan testing strategy** - Unit tests, integration tests, test scripts

## Output

Create a task file at `claude/tasks/[NUMBER]_[snake_case_name].md` with this structure:

```markdown
# Task [NUMBER]: [Feature Name]

## Problem Summary
[What problem does this solve? Why is it needed?]

## Solution Overview
[High-level approach]

## Session-Sized Tasks

| # | Task | Description | Dependencies | Status |
|---|------|-------------|--------------|--------|
| 1 | ... | ... | None | Pending |
| 2 | ... | ... | 1 | Pending |

## Task Details

### Task 1: [Name]
[Detailed description, files to modify, approach]

### Task 2: [Name]
...

## Testing Strategy
[How to verify the implementation]

## Risks & Considerations
[Edge cases, performance concerns, breaking changes]
```

## After Creating the Design

1. Update `claude/prompt.md` to reference the new task
2. Log any significant design decisions in `claude/decisions.md`
3. Present the design summary to the user for approval before implementation

Remember: Do NOT implement yet - this is the design phase only.
