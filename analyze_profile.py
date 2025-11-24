import json
from collections import defaultdict

# Load the profile
with open('benches/cargo 2025-11-24 15.20 profile.json', 'r') as f:
    data = json.load(f)

# Get the shared string array
strings = data['shared']['stringArray']

# Analyze each thread
for thread_idx, thread in enumerate(data['threads']):
    if 'samples' not in thread:
        continue

    samples = thread['samples']
    stack_table = thread.get('stackTable', {})
    frame_table = thread.get('frameTable', {})
    func_table = thread.get('funcTable', {})

    # Get function names for this thread
    func_names = func_table.get('name', [])

    # Get sample stacks (each sample points to a stack frame)
    sample_stacks = samples.get('stack', [])

    # Count samples per function (self-time = top of stack)
    self_counts = defaultdict(int)

    for stack_idx in sample_stacks:
        if stack_idx is not None and stack_idx < len(stack_table.get('frame', [])):
            # Get the frame at the top of this stack
            frame_idx = stack_table['frame'][stack_idx]
            if frame_idx < len(frame_table.get('func', [])):
                func_idx = frame_table['func'][frame_idx]
                if func_idx < len(func_names):
                    name_idx = func_names[func_idx]
                    if name_idx < len(strings):
                        func_name = strings[name_idx]
                        self_counts[func_name] += 1

    # Only print threads with significant samples
    if len(sample_stacks) < 100:
        continue

    # Print results
    print(f'\n=== Thread {thread_idx}: {thread.get("name", "unknown")} ===')
    print(f'Total samples: {len(sample_stacks)}')
    print('\nTop 30 functions by self-time:')

    sorted_funcs = sorted(self_counts.items(), key=lambda x: -x[1])
    for i, (func, count) in enumerate(sorted_funcs[:30], 1):
        pct = (count / len(sample_stacks) * 100) if sample_stacks else 0
        print(f'{i:2}. {count:>5} ({pct:5.1f}%): {func[:90]}')
