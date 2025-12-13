---
trigger: "init", "initialize", "setup"
description: Initialize project with context-engineered architecture
---

# Project Initializer (Context-Engineered)

## Before Starting
Read `.agent/AGENT_RULES.md` to understand the four-layer memory model.

## Step 1: Detect Stack
Look for project files and determine tech stack.

## Step 2: Create Project Files

### feature_list.json
Each feature should be atomic and independently verifiable.

### app_spec.md  
Document architecture, not implementation details.

### init.sh
Minimal script to verify baseline works.

## Step 3: Initialize Memory Layers

### Working Context (.agent/working-context/)
- Will be computed fresh each step
- Never accumulate here

### Sessions (.agent/sessions/)
- Log all events as structured records
- Use: `.agent/hooks/log-event.sh [type] [data]`

### Memory (.agent/memory/)
- Store strategies, constraints, failures, entities
- Use: `.agent/hooks/memory-manager.sh store [category] [content]`

### Artifacts (.agent/artifacts/)
- Large outputs go here, referenced by path
- Use: `.agent/hooks/artifact-manager.sh store [name] [content]`

## Step 4: Record Initial Constraints
```bash
.agent/hooks/capture-feedback.sh constraint "init" "Project constraints..."
```

## Step 5: Compile Initial Context
```bash
.agent/hooks/compile-context.sh
```

## Step 6: Commit
```bash
git add .
git commit -m "chore: initialize context-engineered project"
```
