#!/bin/bash
# Track metrics for feedback loops
# Usage: .agent/hooks/track-metrics.sh <event> <feature_id> [extra_data]

EVENT="$1"
FEATURE_ID="$2"
EXTRA="$3"
METRICS_FILE=".agent/metrics/session-metrics.jsonl"

# Calculate session wall time if start time exists
WALL_TIME=""
if [ -f ".agent/metrics/.session-start" ]; then
    START=$(cat .agent/metrics/.session-start)
    NOW=$(date +%s)
    WALL_TIME=$((NOW - START))
fi

python3 << PYEOF
import json
from datetime import datetime

metrics = {
    "timestamp": datetime.now().isoformat(),
    "event": "$EVENT",
    "feature_id": "$FEATURE_ID",
    "wall_time_seconds": $WALL_TIME if "$WALL_TIME" else None,
    "extra": "$EXTRA" if "$EXTRA" else None
}

# Remove None values
metrics = {k: v for k, v in metrics.items() if v is not None}

with open("$METRICS_FILE", "a") as f:
    f.write(json.dumps(metrics) + "\n")
PYEOF

echo "📈 Tracked: $EVENT for $FEATURE_ID"
