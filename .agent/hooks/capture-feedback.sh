#!/bin/bash
# Capture execution feedback for context evolution
# Implements: Let context evolve through execution (ACE pattern)

OUTCOME="$1"  # success or failure
FEATURE_ID="$2"
DESCRIPTION="$3"

FEEDBACK_DIR=".agent/memory"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)

case "$OUTCOME" in
    success)
        # Capture what worked as a strategy
        mkdir -p "$FEEDBACK_DIR/strategies"
        cat > "$FEEDBACK_DIR/strategies/${FEATURE_ID}-${TIMESTAMP}.md" << STRATEGY
---
feature: $FEATURE_ID
outcome: success
created: $(date -Iseconds)
---
## Strategy That Worked

$DESCRIPTION

### Key Decisions:
- [Extract from description]

### Why It Worked:
- [To be analyzed]

### Reusable Pattern:
- [To be extracted]
STRATEGY
        echo "✅ Strategy captured for future reference"
        ;;
        
    failure)
        # Capture what failed to avoid repetition
        mkdir -p "$FEEDBACK_DIR/failures"
        cat > "$FEEDBACK_DIR/failures/${FEATURE_ID}-${TIMESTAMP}.md" << FAILURE
---
feature: $FEATURE_ID
outcome: failure
created: $(date -Iseconds)
---
## Approach That Failed

$DESCRIPTION

### What Was Tried:
- [Extract from description]

### Why It Failed:
- [To be analyzed]

### Avoid In Future:
- [Extract lesson]
FAILURE
        echo "❌ Failure captured to prevent repetition"
        ;;
        
    constraint)
        # Record active constraint
        mkdir -p "$FEEDBACK_DIR/constraints"
        cat > "$FEEDBACK_DIR/constraints/${FEATURE_ID}-${TIMESTAMP}.md" << CONSTRAINT
---
feature: $FEATURE_ID
type: constraint
active: true
created: $(date -Iseconds)
---
## Active Constraint

$DESCRIPTION
CONSTRAINT
        echo "📌 Constraint recorded"
        ;;
        
    *)
        echo "Usage: capture-feedback.sh [success|failure|constraint] [feature-id] [description]"
        ;;
esac
