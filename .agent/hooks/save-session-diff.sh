#!/bin/bash
# Save a diff summary artifact for the current session
# Usage: .agent/hooks/save-session-diff.sh <session_number> <feature_id>

SESSION_NUM="${1:-unknown}"
FEATURE_ID="${2:-unknown}"
DIFF_DIR=".agent/artifacts/code-snapshots"
mkdir -p "$DIFF_DIR"

DIFF_FILE="$DIFF_DIR/session-${SESSION_NUM}.diff"
SUMMARY_FILE="$DIFF_DIR/session-${SESSION_NUM}-summary.md"

# Get diff since last session tag or last 50 commits
LAST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || git rev-list --max-parents=0 HEAD 2>/dev/null | head -1)

# Save full diff
git diff "$LAST_TAG"..HEAD > "$DIFF_FILE" 2>/dev/null || git diff HEAD~10..HEAD > "$DIFF_FILE" 2>/dev/null

# Generate summary
cat > "$SUMMARY_FILE" << SUMMARY
# Session $SESSION_NUM Diff Summary
Feature: $FEATURE_ID
Generated: $(date -Iseconds)

## Files Changed
$(git diff --stat "$LAST_TAG"..HEAD 2>/dev/null || git diff --stat HEAD~10..HEAD 2>/dev/null)

## Summary
- Lines added: $(git diff --numstat "$LAST_TAG"..HEAD 2>/dev/null | awk '{s+=$1}END{print s+0}')
- Lines removed: $(git diff --numstat "$LAST_TAG"..HEAD 2>/dev/null | awk '{s+=$2}END{print s+0}')
- Files changed: $(git diff --name-only "$LAST_TAG"..HEAD 2>/dev/null | wc -l | tr -d ' ')
SUMMARY

echo "✅ Session diff saved: $DIFF_FILE"
