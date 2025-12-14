---
description: Mark a feature as complete
arguments:
  - name: feature_id
    description: Feature ID (e.g., feat-001)
    required: true
  - name: message
    description: What worked / summary
    required: true
---

Mark feature complete and record what worked:

Feature: $ARGUMENTS.feature_id
Summary: $ARGUMENTS.message

Run: `.agent/commands.sh success "$ARGUMENTS.feature_id" "$ARGUMENTS.message"`
