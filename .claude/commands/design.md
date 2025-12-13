---
allowed-tools: Bash(git:*), Bash(.agent/*:*), Glob, Grep, Read, Write, Edit
argument-hint: <feature-description>
description: Design a feature - analyze codebase and create implementation plan
---

# Feature Design

You are designing a new feature for the AngelScript-Rust project.

## Before Starting

1. Compile working context:
   `.agent/hooks/compile-context.sh`

2. Read current context:
   `.agent/working-context/current.md`

3. Check for known failures to avoid:
   `.agent/commands.sh recall failures`

## Context

1. Check existing tasks to determine the next task number:
   `ls .agent/tasks/*.md | sort -V | tail -5`

2. Check feature_list.json for parent feature (if adding a subtask):
   `feature_list.json`

3. Check git status for any in-progress changes:
   `git status --short`

## Your Task

Design the following feature: **$ARGUMENTS**

## Design Process

1. **Analyze the codebase** - Search for relevant code, understand existing patterns
2. **Identify affected components** - Which crates, modules, files need changes
3. **Determine if subtask or new feature** - Is this part of an existing feature (e.g., compiler) or standalone?
4. **Break into session-sized tasks** - Each task completable in one session without context overflow
5. **Consider edge cases** - Error handling, backwards compatibility, performance
6. **Plan testing strategy** - Unit tests, integration tests, test scripts

## Output

Create a task file at `.agent/tasks/[NUMBER]_[snake_case_name].md` with this structure:

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

1. **Update feature_list.json**:
   - If new feature: Add to top-level `features` array
   - If subtask of existing feature: Add to parent's `subtasks` array
   - Set status to "pending"
   - Include task file path

2. Store design decisions in memory:
   `.agent/hooks/memory-manager.sh store strategies "[key decisions made]"`

3. Log the design event:
   `.agent/hooks/log-event.sh design "[task-number]: [feature-name]"`

4. Present the design summary to the user for approval before implementation

Remember: Do NOT implement yet - this is the design phase only.
