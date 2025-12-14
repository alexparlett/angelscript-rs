#!/usr/bin/env python3
"""
PreCompact Hook - Saves state before context compaction.

Fires when:
- User runs /compact
- Auto-compaction triggers (context too large)

Saves a snapshot so SessionStart can restore context.

IMPORTANT: This hook is READ-ONLY except for writing to .agent/sessions/.
It does not modify project files or make network requests.

Requires: Claude Code 1.0.17+ (with PreCompact hook support)
"""

import json
import sys
from pathlib import Path
from datetime import datetime

# Maximum snapshots to keep (older ones are cleaned up)
MAX_SNAPSHOTS = 10

# Maximum snapshot size in characters
MAX_SNAPSHOT_SIZE = 50000

def cleanup_old_snapshots(snapshot_dir):
    """
    Remove old snapshots, keeping only the most recent MAX_SNAPSHOTS.
    
    Snapshots are named pre-compact-YYYYMMDD-HHMMSS.md which sorts
    lexicographically by timestamp (newest = highest string value).
    """
    try:
        snapshots = list(snapshot_dir.glob("pre-compact-*.md"))
        
        # Sort by filename (lexicographic = chronological for our timestamp format)
        # Reverse to get newest first
        snapshots.sort(key=lambda p: p.name, reverse=True)
        
        # Delete everything beyond MAX_SNAPSHOTS
        for old in snapshots[MAX_SNAPSHOTS:]:
            try:
                old.unlink()
            except Exception:
                pass  # Ignore deletion failures
    except Exception:
        pass

def save_pre_compact_state():
    """Save current state before compaction."""
    
    # Create snapshot directory
    snapshot_dir = Path(".agent/sessions/snapshots")
    snapshot_dir.mkdir(parents=True, exist_ok=True)
    
    timestamp = datetime.now().strftime("%Y%m%d-%H%M%S")
    snapshot_file = None
    
    # Save current working context if it exists
    current_context = Path(".agent/working-context/current.md")
    if current_context.exists():
        try:
            content = current_context.read_text()
            
            # Truncate if too large
            if len(content) > MAX_SNAPSHOT_SIZE:
                content = content[:MAX_SNAPSHOT_SIZE] + "\n\n[Truncated]"
            
            snapshot_file = snapshot_dir / f"pre-compact-{timestamp}.md"
            snapshot_file.write_text(content)
        except Exception:
            pass
    
    # Log the compaction event
    try:
        log_file = Path(".agent/sessions/compact-log.jsonl")
        log_entry = {
            "timestamp": datetime.now().isoformat(),
            "event": "pre_compact",
            "snapshot": str(snapshot_file) if snapshot_file else None
        }
        
        with open(log_file, "a") as f:
            f.write(json.dumps(log_entry) + "\n")
    except Exception:
        pass
    
    # Cleanup old snapshots
    cleanup_old_snapshots(snapshot_dir)
    
    return timestamp

def main():
    """
    Main entry point for PreCompact hook.
    
    PreCompact cannot block or inject context - it's informational only.
    """
    try:
        # Read input from Claude Code
        input_data = json.load(sys.stdin)
        
        trigger = input_data.get("trigger", "unknown")  # manual or auto
        
        # Save state
        timestamp = save_pre_compact_state()
        
        # Log to stderr (shown to user)
        print(f"💾 Context snapshot saved: pre-compact-{timestamp}.md", file=sys.stderr)
        
        if trigger == "auto":
            print("⚠️  Auto-compaction triggered (context was large)", file=sys.stderr)
        
    except Exception as e:
        print(f"⚠️ PreCompact hook error: {e}", file=sys.stderr)
    
    # PreCompact doesn't support additionalContext - output empty
    print(json.dumps({}))

if __name__ == "__main__":
    main()
