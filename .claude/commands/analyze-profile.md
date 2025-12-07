---
allowed-tools: Bash(python3:*), Read
argument-hint: <path/to/profile.json>
description: Analyze a samply profile JSON for hot spots
---

Analyze a samply/Firefox Profiler JSON to find performance hot spots.

Read the profile analysis instructions from `/claude/profile_analysis_instructions.md` for context.

The user has provided a profile at: $ARGUMENTS

Steps:
1. Read the profile JSON file
2. Parse it to find the benchmark thread (usually `module_benchmarks` or similar with the most samples)
3. Extract top functions by self-time using the Python script from the instructions
4. Report:
   - Thread name and total samples
   - Top 20 functions by self-time with percentages
   - Group by category (Lexer, Parser, System calls like malloc/memmove)
   - Identify the main bottlenecks

Use this Python analysis script:
```python
import json
from collections import defaultdict

with open('$ARGUMENTS', 'r') as f:
    data = json.load(f)

strings = data['shared']['stringArray']

for thread_idx, thread in enumerate(data['threads']):
    if 'samples' not in thread:
        continue
    samples = thread['samples']
    stack_table = thread.get('stackTable', {})
    frame_table = thread.get('frameTable', {})
    func_table = thread.get('funcTable', {})
    func_names = func_table.get('name', [])
    sample_stacks = samples.get('stack', [])
    self_counts = defaultdict(int)
    for stack_idx in sample_stacks:
        if stack_idx is not None and stack_idx < len(stack_table.get('frame', [])):
            frame_idx = stack_table['frame'][stack_idx]
            if frame_idx < len(frame_table.get('func', [])):
                func_idx = frame_table['func'][frame_idx]
                if func_idx < len(func_names):
                    name_idx = func_names[func_idx]
                    if name_idx < len(strings):
                        self_counts[strings[name_idx]] += 1
    if len(sample_stacks) > 100:
        print(f'\n=== Thread {thread_idx}: {thread.get("name", "unknown")} ===')
        print(f'Total samples: {len(sample_stacks)}')
        sorted_funcs = sorted(self_counts.items(), key=lambda x: -x[1])
        for i, (func, count) in enumerate(sorted_funcs[:30], 1):
            pct = count / len(sample_stacks) * 100
            print(f'{i:2}. {count:>5} ({pct:5.1f}%): {func[:90]}')
```
