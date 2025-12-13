# Claude Code Instructions
<!-- harness_version: 3.1.0 -->
<!-- generated: 2025-12-13T01:09:05+00:00 -->

## Context Engineering
This project uses a four-layer memory architecture. Read \`.agent/AGENT_RULES.md\`.

## Before Each Step
\`\`\`bash
.agent/hooks/compile-context.sh
cat .agent/working-context/current.md
.agent/commands.sh recall failures  # Don't repeat mistakes!
\`\`\`

## ⚠️ MANDATORY: Use MCP Tools for Documentation
If this project has MCP configured (check mcp-config.json), you MUST use MCP tools:

### Ref Documentation (if available)
Before implementing anything with external libraries, query Ref for current docs:
\`\`\`
Use the Ref MCP tool to look up documentation for [library/crate/package]
\`\`\`

Examples:
- "Use Ref to look up axum router documentation"
- "Use Ref to look up sqlx query macro syntax"
- "Use Ref to look up russh SSH client examples"

**DO NOT guess at APIs. Look them up first.**

### Other MCP Tools
- **fetch** - Make HTTP requests to test endpoints
- **postgres** - Query the database directly
- **filesystem** - Read/write files outside the project

## During Implementation
- Log events: `.agent/hooks/log-event.sh [type] [data]`
- Store large outputs: `.agent/hooks/artifact-manager.sh store [name]`
- Retrieve memory: `.agent/hooks/memory-manager.sh retrieve [category]`

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

## Key Principles
1. Context is computed, not accumulated
2. Store large outputs as artifacts, reference by path
3. Retrieve memory on demand, don't pin everything
4. **USE MCP** - look up docs before guessing at APIs
5. **RUN TESTS** - no exceptions
6. **USE SUBAGENTS** - code-reviewer, test-runner, feature-verifier
7. Capture feedback for context evolution

## Search Guidelines
Do NOT include years in documentation searches.