#!/bin/bash
# Context Compiler - Assembles minimal working context for each step
# Implements: Computed context, schema-driven summarization, retrieval

WORKING_CONTEXT=".agent/working-context/current.md"
SESSION_LOG=".agent/sessions/current.jsonl"

echo "🔧 Compiling working context..."

# Start fresh (context is computed, not accumulated)
cat > "$WORKING_CONTEXT" << CONTEXT
# Working Context

## Current Task
CONTEXT

# Add current feature (from feature_list.json)
if [ -f "feature_list.json" ]; then
    echo '```json' >> "$WORKING_CONTEXT"
    # Get only the current/next feature, not all
    python3 -c "
import json
with open('feature_list.json') as f:
    data = json.load(f)
for feat in sorted(data.get('features', []), key=lambda x: x.get('priority', 99)):
    if not feat.get('passes', False):
        print(json.dumps(feat, indent=2))
        break
" >> "$WORKING_CONTEXT" 2>/dev/null || echo "No pending features"
    echo '```' >> "$WORKING_CONTEXT"
fi

# Add active constraints from memory (retrieved, not pinned)
echo "" >> "$WORKING_CONTEXT"
echo "## Active Constraints" >> "$WORKING_CONTEXT"
if [ -d ".agent/memory/constraints" ]; then
    for constraint in .agent/memory/constraints/*.md; do
        [ -f "$constraint" ] && cat "$constraint" >> "$WORKING_CONTEXT"
    done
fi

# Add relevant failures (avoid repeating mistakes)
echo "" >> "$WORKING_CONTEXT"
echo "## Known Failures (Don't Repeat)" >> "$WORKING_CONTEXT"
if [ -d ".agent/memory/failures" ]; then
    # Only recent failures, not all
    ls -t .agent/memory/failures/*.md 2>/dev/null | head -5 | while read f; do
        [ -f "$f" ] && cat "$f" >> "$WORKING_CONTEXT"
    done
fi

# Add relevant strategies (what worked)
echo "" >> "$WORKING_CONTEXT"
echo "## Working Strategies" >> "$WORKING_CONTEXT"
if [ -d ".agent/memory/strategies" ]; then
    ls -t .agent/memory/strategies/*.md 2>/dev/null | head -3 | while read f; do
        [ -f "$f" ] && cat "$f" >> "$WORKING_CONTEXT"
    done
fi

# Reference artifacts by path (not content!)
echo "" >> "$WORKING_CONTEXT"
echo "## Available Artifacts (fetch if needed)" >> "$WORKING_CONTEXT"
if [ -d ".agent/artifacts/tool-outputs" ]; then
    ls .agent/artifacts/tool-outputs/ 2>/dev/null | head -10 | while read f; do
        echo "- .agent/artifacts/tool-outputs/$f" >> "$WORKING_CONTEXT"
    done
fi

# Add recent session summary (compressed, not raw)
echo "" >> "$WORKING_CONTEXT"
echo "## Recent Session Summary" >> "$WORKING_CONTEXT"
if [ -f "agent-progress.txt" ]; then
    tail -30 agent-progress.txt >> "$WORKING_CONTEXT"
fi

# ============================================================================
# Context Budget Enforcement
# ============================================================================
MAX_TOKENS=8000  # Hard cap for context budget

# Calculate token estimate
TOKENS=$(wc -w "$WORKING_CONTEXT" | awk '{print int($1 * 1.3)}')

# If over budget, trim content
if [ "$TOKENS" -gt "$MAX_TOKENS" ]; then
    echo "⚠️  Context over budget ($TOKENS > $MAX_TOKENS tokens), trimming..."
    
    # Create trimmed version
    TRIMMED_CONTEXT=".agent/working-context/trimmed.md"
    
    # Keep header and current task (most important)
    head -50 "$WORKING_CONTEXT" > "$TRIMMED_CONTEXT"
    
    # Add only most recent failures (last 3 instead of 5)
    echo "" >> "$TRIMMED_CONTEXT"
    echo "## Known Failures (Trimmed - Last 3)" >> "$TRIMMED_CONTEXT"
    if [ -d ".agent/memory/failures" ]; then
        ls -t .agent/memory/failures/*.md 2>/dev/null | head -3 | while read f; do
            [ -f "$f" ] && head -20 "$f" >> "$TRIMMED_CONTEXT"
        done
    fi
    
    # Add only most recent strategies (last 2)
    echo "" >> "$TRIMMED_CONTEXT"
    echo "## Working Strategies (Trimmed)" >> "$TRIMMED_CONTEXT"
    if [ -d ".agent/memory/strategies" ]; then
        ls -t .agent/memory/strategies/*.md 2>/dev/null | head -2 | while read f; do
            [ -f "$f" ] && head -15 "$f" >> "$TRIMMED_CONTEXT"
        done
    fi
    
    # Truncate session summary
    echo "" >> "$TRIMMED_CONTEXT"
    echo "## Recent Session Summary (Trimmed)" >> "$TRIMMED_CONTEXT"
    if [ -f "agent-progress.txt" ]; then
        tail -15 agent-progress.txt >> "$TRIMMED_CONTEXT"
    fi
    
    # Replace with trimmed version
    mv "$TRIMMED_CONTEXT" "$WORKING_CONTEXT"
    TOKENS=$(wc -w "$WORKING_CONTEXT" | awk '{print int($1 * 1.3)}')
    echo "   Trimmed to ~$TOKENS tokens"
fi

echo "" >> "$WORKING_CONTEXT"
echo "---" >> "$WORKING_CONTEXT"
echo "Estimated tokens: ~$TOKENS" >> "$WORKING_CONTEXT"
echo "Compiled: $(date -Iseconds)" >> "$WORKING_CONTEXT"

echo "✅ Working context compiled: $WORKING_CONTEXT (~$TOKENS tokens)"
