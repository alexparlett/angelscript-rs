---
description: Record a failure to memory
arguments:
  - name: feature_id
    description: Feature ID (e.g., feat-001)
    required: true
  - name: message
    description: What went wrong
    required: true
---

Record this failure to `.agent/memory/failures/` so we don't repeat it:

Feature: $ARGUMENTS.feature_id
Issue: $ARGUMENTS.message

Run: `.agent/commands.sh failure "$ARGUMENTS.feature_id" "$ARGUMENTS.message"`
