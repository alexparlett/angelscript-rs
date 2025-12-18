#!/usr/bin/env python3
"""
PreCompact Hook - Saves state before context compaction.

Fires when:
- User runs /compact
- Auto-compaction triggers (context too large)

Saves a snapshot of the conversation so SessionStart can restore context.

IMPORTANT: This hook is READ-ONLY except for writing to .agent/sessions/.
It does not modify project files or make network requests.

Requires: Claude Code 1.0.17+ (with PreCompact hook support)
"""

import json
import sys
from pathlib import Path
from datetime import datetime
import re

# Maximum snapshots to keep (older ones are cleaned up)
MAX_SNAPSHOTS = 10

# Maximum snapshot size in characters
MAX_SNAPSHOT_SIZE = 50000

# Maximum messages to include in summary
MAX_MESSAGES = 20

# Keywords that indicate important content
IMPORTANT_KEYWORDS = [
    'implemented', 'fixed', 'error', 'failed', 'decided', 'found',
    'issue', 'problem', 'solution', 'because', 'changed', 'updated',
    'added', 'removed', 'created', 'modified'
]


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


def extract_transcript_summary(transcript_path):
    """
    Extract a summary from the conversation transcript.

    Returns a markdown summary of recent messages and key points.
    """
    try:
        transcript_file = Path(transcript_path).expanduser()
        if not transcript_file.exists():
            return None

        messages = []
        with open(transcript_file, 'r') as f:
            for line in f:
                try:
                    msg = json.loads(line.strip())
                    messages.append(msg)
                except json.JSONDecodeError:
                    continue

        if not messages:
            return None

        # Take the last N messages
        recent = messages[-MAX_MESSAGES:]

        summary_parts = ["# Session Summary (Pre-Compact)\n"]

        # Extract key exchanges
        for msg in recent:
            # Handle different transcript formats:
            # Format 1: {"type": "user/assistant", "message": {"role": ..., "content": ...}}
            # Format 2: {"role": "user/assistant", "content": ...}
            msg_type = msg.get('type', '')

            # Skip non-message entries (queue-operation, etc.)
            if msg_type not in ('user', 'assistant'):
                # Try legacy format
                role = msg.get('role', '')
                if role not in ('user', 'assistant'):
                    continue
                content = msg.get('content', '')
            else:
                role = msg_type
                # Content is nested in 'message' field
                inner_msg = msg.get('message', {})
                content = inner_msg.get('content', '')

            # Handle content that might be a list of blocks
            if isinstance(content, list):
                text_parts = []
                for block in content:
                    if isinstance(block, dict) and block.get('type') == 'text':
                        text_parts.append(block.get('text', ''))
                content = '\n'.join(text_parts)

            if not content or len(content) < 10:
                continue

            # Truncate long messages
            if len(content) > 500:
                # Try to find a good break point
                content = content[:500] + "..."

            if role == 'user':
                summary_parts.append(f"\n**User:** {content}\n")
            elif role == 'assistant':
                # Check if this message has important content
                content_lower = content.lower()
                has_important = any(kw in content_lower for kw in IMPORTANT_KEYWORDS)

                if has_important or len(content) > 100:
                    summary_parts.append(f"\n**Assistant:** {content}\n")

        summary = '\n'.join(summary_parts)

        # Truncate if too large
        if len(summary) > MAX_SNAPSHOT_SIZE:
            summary = summary[:MAX_SNAPSHOT_SIZE] + "\n\n[Truncated]"

        return summary

    except Exception as e:
        return f"# Session Summary\n\nError extracting transcript: {e}"


def get_current_task_context():
    """Get the current task and project state context."""
    parts = []

    # Current task from feature_list.json
    feature_file = Path("feature_list.json")
    if feature_file.exists():
        try:
            import json as json_module
            with open(feature_file) as f:
                data = json_module.load(f)

            next_task_id = data.get("next_task")
            features = data.get("features", [])

            # Find the current task
            def find_task(items, target_id):
                for item in items:
                    if item.get("id") == target_id:
                        return item
                    subtasks = item.get("subtasks", [])
                    if subtasks:
                        found = find_task(subtasks, target_id)
                        if found:
                            return found
                return None

            task = find_task(features, next_task_id) if next_task_id else None
            if task:
                parts.append(f"## Current Task\n**{task.get('id')}**: {task.get('name')}\n{task.get('description', '')[:200]}")
        except Exception:
            pass

    # Recent failures (important context)
    failures_dir = Path(".agent/memory/failures")
    if failures_dir.exists():
        failure_files = sorted(
            failures_dir.glob("*.md"),
            key=lambda x: x.stat().st_mtime,
            reverse=True
        )[:2]
        if failure_files:
            parts.append("## Recent Failures")
            for f in failure_files:
                try:
                    content = f.read_text()[:300]
                    parts.append(content.strip())
                except Exception:
                    pass

    # Active constraints
    constraints_dir = Path(".agent/memory/constraints")
    if constraints_dir.exists():
        constraint_files = list(constraints_dir.glob("*.md"))[:2]
        if constraint_files:
            parts.append("## Constraints")
            for c in constraint_files:
                try:
                    content = c.read_text()[:200]
                    parts.append(content.strip())
                except Exception:
                    pass

    return "\n\n".join(parts) if parts else None


def save_pre_compact_state(transcript_path=None):
    """Save current state before compaction."""

    # Create snapshot directory
    snapshot_dir = Path(".agent/sessions/snapshots")
    snapshot_dir.mkdir(parents=True, exist_ok=True)

    timestamp = datetime.now().strftime("%Y%m%d-%H%M%S")
    snapshot_file = None

    # Build comprehensive snapshot with both project state AND conversation
    content_parts = []

    # 1. Add project state context (task, failures, constraints)
    project_context = get_current_task_context()
    if project_context:
        content_parts.append(project_context)

    # 2. Add conversation transcript summary
    if transcript_path:
        transcript_summary = extract_transcript_summary(transcript_path)
        if transcript_summary:
            content_parts.append(transcript_summary)

    # 3. Fall back to current.md if nothing else
    if not content_parts:
        current_context = Path(".agent/working-context/current.md")
        if current_context.exists():
            try:
                content_parts.append(current_context.read_text())
            except Exception:
                pass

    content = "\n\n---\n\n".join(content_parts) if content_parts else None

    # Save the snapshot
    if content:
        try:
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
            "snapshot": str(snapshot_file) if snapshot_file else None,
            "had_transcript": transcript_path is not None
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
        transcript_path = input_data.get("transcript_path")  # Path to conversation JSONL

        # Save state (pass transcript for extraction)
        timestamp = save_pre_compact_state(transcript_path)

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
