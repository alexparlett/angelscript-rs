#!/bin/bash
# Context Engine Commands (Native Hooks Mode)

MEMORY_DIR=".agent/memory"
mkdir -p "$MEMORY_DIR"/{failures,strategies,constraints}

case "$1" in
    recall)
        # Recall from memory
        CATEGORY="${2:-failures}"
        echo "=== Recalling: $CATEGORY ==="
        for f in "$MEMORY_DIR/$CATEGORY"/*.md; do
            [ -f "$f" ] && cat "$f"
        done
        ;;
    
    success)
        # Mark feature as complete
        FEATURE_ID="$2"
        MESSAGE="$3"
        
        if [ -f "feature_list.json" ]; then
            python3 -c "
import json
with open('feature_list.json') as f:
    data = json.load(f)
for feat in data.get('features', []):
    if feat.get('id') == '$FEATURE_ID':
        feat['passes'] = True
        break
with open('feature_list.json', 'w') as f:
    json.dump(data, f, indent=2)
print('✅ Marked $FEATURE_ID as complete')
"
        fi
        
        # Record to strategies
        TIMESTAMP=$(date +%Y%m%d-%H%M%S)
        echo "# Success: $FEATURE_ID" > "$MEMORY_DIR/strategies/$TIMESTAMP.md"
        echo "$MESSAGE" >> "$MEMORY_DIR/strategies/$TIMESTAMP.md"
        ;;
    
    failure)
        # Record failure
        FEATURE_ID="$2"
        MESSAGE="$3"
        
        TIMESTAMP=$(date +%Y%m%d-%H%M%S)
        echo "# Failure: $FEATURE_ID" > "$MEMORY_DIR/failures/$TIMESTAMP.md"
        echo "$MESSAGE" >> "$MEMORY_DIR/failures/$TIMESTAMP.md"
        echo "❌ Recorded failure for $FEATURE_ID"
        ;;
    
    compile)
        # Manually compile context
        python3 .claude/hooks/session-start.py < /dev/null 2>/dev/null | python3 -c "
import json, sys
data = json.load(sys.stdin)
ctx = data.get('hookSpecificOutput', {}).get('additionalContext', '')
print(ctx)
" > .agent/working-context/current.md
        echo "✅ Context compiled to .agent/working-context/current.md"
        ;;
    
    status)
        # Show feature status
        if [ -f "feature_list.json" ]; then
            python3 -c "
import json
with open('feature_list.json') as f:
    data = json.load(f)
features = data.get('features', [])
completed = sum(1 for f in features if f.get('passes', False))
blocked = sum(1 for f in features if f.get('blocked', False))
total = len(features)
print(f'Features: {completed}/{total} completed, {blocked} blocked')
for f in features:
    if not f.get('passes') and not f.get('blocked'):
        print(f\"  Next: {f.get('id')} - {f.get('name')}\")
        break
"
        else
            echo "No feature_list.json found"
        fi
        ;;
    
    *)
        echo "Usage: .agent/commands.sh <command>"
        echo ""
        echo "Commands:"
        echo "  recall [category]  - Recall from memory (failures/strategies/constraints)"
        echo "  success <id> <msg> - Mark feature complete"
        echo "  failure <id> <msg> - Record failure"
        echo "  compile            - Manually compile context"
        echo "  status             - Show feature status"
        ;;
esac
