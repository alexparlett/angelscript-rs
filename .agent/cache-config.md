# Cache Stability Guidelines

KV-cache hit rate is critical for cost and latency.
Cached tokens: $0.30/M vs Uncached: $3.00/M (10x difference)

## Rules for Cache Stability

### 1. Stable Prefix
The system prompt should be IDENTICAL across calls.

❌ Wrong:
```
Current time: 2025-01-15T14:32:15Z  # Changes every call!
You are a helpful assistant...
```

✅ Right:
```
You are a helpful assistant...
[stable instructions]
---
[variable content below the break]
```

### 2. Append-Only History
Don't modify previous messages. Only append new ones.

### 3. Deterministic Serialization
Ensure JSON keys are always in same order.
```python
import json
json.dumps(data, sort_keys=True)  # Always sorted
```

### 4. Separate Stable from Variable
```
[STABLE PREFIX - cached]
System instructions
Agent identity
Long-lived summaries

[VARIABLE SUFFIX - not cached]
Current task
New tool outputs
Latest input
```

### 5. Mark Cache Breakpoints
If using explicit caching, ensure breakpoints cover system prompt.
