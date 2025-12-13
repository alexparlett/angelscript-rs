#!/bin/bash
# Log structured event to session (Layer 2)
# Events are typed records, not raw prompts

SESSION_LOG=".agent/sessions/current.jsonl"
EVENT_TYPE="$1"
EVENT_DATA="$2"

# Create session file if doesn't exist
[ ! -f "$SESSION_LOG" ] && echo "" > "$SESSION_LOG"

# Log structured event
python3 -c "
import json
import datetime

event = {
    'timestamp': datetime.datetime.now().isoformat(),
    'type': '$EVENT_TYPE',
    'data': '''$EVENT_DATA''',
    'session_id': '$(date +%Y%m%d)'
}
print(json.dumps(event))
" >> "$SESSION_LOG"

echo "📝 Logged event: $EVENT_TYPE"
