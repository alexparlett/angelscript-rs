# Claude Code Instructions

## Context Engineering
This project uses Claude Code native hooks for automatic context management.
- **SessionStart**: Injects project context on new session, resume, or /clear
- **PreCompact**: Saves conversation summary before compaction

## Project Structure
- `tasks/` - Task definitions (committed)
- `feature_list.json` - Task progress and next_task pointer
- `.agent/memory/` - Local state (constraints, failures, strategies)
- `.agent-template/` - Bootstrap template for new worktrees (committed)

## ⚠️ MANDATORY: Run Tests Before Completing Any Feature
You MUST run tests before marking any feature as complete:

```bash
# Rust
cargo test

# Python  
pytest

# Node.js
npm test

# Go
go test ./...
```

**Do NOT set `passes: true` unless tests actually pass!**

## ⚠️ MANDATORY: Use Subagents
After implementing a feature, you MUST invoke these subagents:

### 1. Code Review (REQUIRED)
```
@code-reviewer Review the changes for this feature
```
Wait for the review. Address any issues before proceeding.

### 2. Test Runner (REQUIRED)
```
@test-runner Run the test suite and analyze results
```
If tests fail, fix them before proceeding.

### 3. Feature Verifier (REQUIRED)
```
@feature-verifier Verify feature [ID]: [description]
```
Confirm the feature works end-to-end.

**Do NOT skip subagents. They catch issues before they compound.**

## After Completing Work
```bash
# Only if tests pass and verification succeeds!
.agent/commands.sh success "[feature-id]" "what worked"
git add -A
git commit -m "session: completed [feature-id]"
```

If something fails:
```bash
.agent/commands.sh failure "[feature-id]" "what failed and why"
```

## Commands
- `.agent/commands.sh status` - Check progress
- `.agent/commands.sh success <id> <msg>` - Mark feature complete
- `.agent/commands.sh failure <id> <msg>` - Record failure (don't repeat!)
- `.agent/commands.sh recall failures` - See past failures

## Key Principles
1. **RUN TESTS** - no exceptions
2. **USE SUBAGENTS** - code-reviewer, test-runner, feature-verifier
3. Record failures so they're not repeated
