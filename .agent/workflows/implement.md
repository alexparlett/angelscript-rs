---
trigger: "implement", "next", "continue", "build"
description: Implement features using context engineering principles
---

# Feature Implementation (Context-Engineered)

## Critical Principles
1. Context is COMPUTED, not accumulated
2. Retrieve what's needed, don't pin everything
3. Offload large outputs to artifacts
4. Capture feedback for evolution

## Session Startup

### 1. Compile Fresh Working Context
```bash
.agent/hooks/compile-context.sh
```
This gives you minimal relevant context, not full history.

### 2. Review Working Context
```bash
cat .agent/working-context/current.md
```
This is what you're working with. If something's missing, retrieve it explicitly.

### 3. Verify Baseline
```bash
./init.sh
```

### 4. If Broken, Revert
```bash
GOOD=$(git log --oneline --grep="session:" -1 --format="%H")
git stash && git reset --hard $GOOD
```

## During Implementation

### Log Significant Events
```bash
.agent/hooks/log-event.sh "tool_call" "description of what was done"
.agent/hooks/log-event.sh "decision" "what was decided and why"
.agent/hooks/log-event.sh "error" "what went wrong"
```

### Store Large Outputs as Artifacts
Don't paste large tool outputs into conversation. Instead:
```bash
echo "[large output]" | .agent/hooks/artifact-manager.sh store "output-name.txt"
```
Then reference by path: `.agent/artifacts/tool-outputs/output-name.txt`

### Retrieve Memory When Needed
```bash
.agent/hooks/memory-manager.sh retrieve strategies
.agent/hooks/memory-manager.sh retrieve failures
.agent/hooks/memory-manager.sh search "relevant term"
```

### Commit Frequently
```bash
git add -A && git commit -m "feat(category): description"
```

## Session End

### 1. Capture What Worked
```bash
.agent/hooks/capture-feedback.sh success "feature-id" "Description of approach that worked"
```

### 2. Capture What Failed (if applicable)
```bash
.agent/hooks/capture-feedback.sh failure "feature-id" "Description of what didn't work and why"
```

### 3. Record New Constraints
```bash
.agent/hooks/capture-feedback.sh constraint "feature-id" "New constraint discovered"
```

### 4. Update Feature List
Only if verified working.

### 5. Update Progress Log
Include schema-preserving summary:
- Causal steps taken
- Active constraints
- What failed
- What's committed to next

### 6. Final Commit
```bash
git add -A && git commit -m "session: completed [feature-id]"
```

### 7. Recompile Context for Next Session
```bash
.agent/hooks/compile-context.sh
```

## Key Reminders

❌ Don't: Accumulate everything in context
✅ Do: Compute minimal working context per step

❌ Don't: Paste large outputs into conversation
✅ Do: Store as artifacts, reference by path

❌ Don't: Summarize blindly to save space
✅ Do: Use schema-driven summarization

❌ Don't: Forget what failed
✅ Do: Capture failures to prevent repetition

❌ Don't: Start fresh every session
✅ Do: Let context evolve through feedback
