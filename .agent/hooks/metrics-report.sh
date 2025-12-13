#!/bin/bash
# Generate metrics report
# Usage: .agent/hooks/metrics-report.sh

METRICS_FILE=".agent/metrics/session-metrics.jsonl"

if [ ! -f "$METRICS_FILE" ]; then
    echo "No metrics collected yet"
    exit 0
fi

python3 << 'PYEOF'
import json
from collections import defaultdict

metrics_file = ".agent/metrics/session-metrics.jsonl"
events = []

with open(metrics_file) as f:
    for line in f:
        line = line.strip()
        if line:
            try:
                events.append(json.loads(line))
            except:
                pass

if not events:
    print("No metrics collected yet")
    exit(0)

# Aggregate metrics
total_sessions = len([e for e in events if e.get("event") == "session_complete"])
total_features = len([e for e in events if e.get("event") == "feature_complete"])
retries = len([e for e in events if e.get("event") == "retry"])
reverts = len([e for e in events if e.get("event") == "revert"])

wall_times = [e.get("wall_time_seconds", 0) for e in events if e.get("wall_time_seconds")]
avg_wall_time = sum(wall_times) / len(wall_times) if wall_times else 0

# Per-feature stats
feature_attempts = defaultdict(int)
for e in events:
    if e.get("event") in ("session_start", "retry"):
        feature_attempts[e.get("feature_id", "unknown")] += 1

flaky_features = [f for f, count in feature_attempts.items() if count > 2]

print("=" * 50)
print("📊 METRICS REPORT")
print("=" * 50)
print(f"Total sessions:     {total_sessions}")
print(f"Features completed: {total_features}")
print(f"Retries:            {retries}")
print(f"Reverts:            {reverts}")
print(f"Avg wall time:      {avg_wall_time:.1f}s")
print(f"Flaky features:     {len(flaky_features)}")
if flaky_features:
    print(f"  - {', '.join(flaky_features[:5])}")
print("=" * 50)
PYEOF
