#!/usr/bin/env python3
"""
SessionStart Hook - Injects compiled context into Claude's window.

Fires when:
- Claude Code starts a new session
- Claude Code resumes an existing session
- After /clear command

Outputs additionalContext that gets injected into Claude's context.

IMPORTANT: This hook is READ-ONLY. It only reads from .agent/ and outputs context.
It does not modify files or make network requests.

Requires: Claude Code 1.0.17+ (with SessionStart hook support)
"""

import json
import sys
import os
from pathlib import Path

# ============================================================================
# Configuration
# ============================================================================

# Maximum characters for additionalContext to prevent silent truncation
# Claude Code's limit is ~10k chars; we stay well under
MAX_CONTEXT_CHARS = 6000

# Section priorities for truncation (higher = keep longer)
# When over budget, we trim lowest priority sections first
SECTION_PRIORITIES = {
    "header": 100,        # Always keep
    "current_task": 90,   # Critical - what to work on
    "session_summary": 85, # Important - context from pre-compact
    "constraints": 80,    # Important - project rules
    "failures": 70,       # Important - don't repeat mistakes
    "commands": 65,       # Important - failure recording instructions
    "strategies": 60,     # Helpful but expendable
}

# Maximum items to include from each memory category
MAX_FAILURES = 3
MAX_STRATEGIES = 2
MAX_CONSTRAINTS = 3
MAX_SESSION_SUMMARY_CHARS = 1500  # Keep session summaries concise

# Truncation limits per item
FAILURE_CHAR_LIMIT = 400
STRATEGY_CHAR_LIMIT = 300
CONSTRAINT_CHAR_LIMIT = 200

def bootstrap_agent_directory():
    """
    Initialize .agent/ from .agent-template/ if it doesn't exist.

    This ensures new worktrees have the base context (constraints,
    strategies, permanent failures) without manual setup.
    """
    agent_dir = Path(".agent")
    template_dir = Path(".agent-template")

    # Only bootstrap if .agent doesn't exist but template does
    if agent_dir.exists() or not template_dir.exists():
        return False

    try:
        import shutil
        shutil.copytree(template_dir, agent_dir)
        return True
    except Exception as e:
        # Log but don't fail - hooks should be resilient
        print(f"⚠️ Could not bootstrap .agent/: {e}", file=sys.stderr)
        return False


def estimate_tokens(text):
    """Rough token estimate (1.3 tokens per word)."""
    return int(len(text.split()) * 1.3)

def build_section(name, content):
    """Build a section with metadata for priority-based truncation."""
    return {
        "name": name,
        "priority": SECTION_PRIORITIES.get(name, 0),
        "content": content,
        "size": len(content)
    }

def truncate_by_priority(sections, max_chars):
    """
    Truncate sections by priority to fit within max_chars.

    Strategy: Remove lowest-priority sections first, then truncate
    remaining sections if still over budget.
    """
    # Sort by priority (lowest first for removal)
    sorted_sections = sorted(sections, key=lambda s: s["priority"])

    total_size = sum(s["size"] for s in sections)

    # Remove lowest-priority sections until under budget
    while total_size > max_chars and sorted_sections:
        # Check if removing the lowest priority section helps
        lowest = sorted_sections[0]
        if lowest["priority"] < 80:  # Don't remove critical sections
            sorted_sections.pop(0)
            total_size -= lowest["size"]
        else:
            break

    # If still over, truncate the remaining sections proportionally
    if total_size > max_chars:
        ratio = max_chars / total_size
        for section in sorted_sections:
            max_section_size = int(section["size"] * ratio * 0.9)  # 10% safety margin
            if len(section["content"]) > max_section_size:
                # Truncate at a clean line break
                truncated = section["content"][:max_section_size]
                last_newline = truncated.rfind("\n")
                if last_newline > max_section_size // 2:
                    truncated = truncated[:last_newline]
                section["content"] = truncated + "\n[...truncated]"

    # Reassemble in original priority order (highest first for output)
    sorted_sections.sort(key=lambda s: s["priority"], reverse=True)
    return "\n\n".join(s["content"] for s in sorted_sections if s["content"].strip())

def is_complete(item):
    """Check if a feature/subtask is complete (handles both schemas)."""
    if item.get("passes", False):
        return True
    return item.get("status", "").lower() == "complete"

def is_blocked(item):
    """Check if a feature/subtask is blocked."""
    if item.get("blocked", False):
        return True
    return item.get("status", "").lower() == "blocked"

def count_tasks(items):
    """Count completed and total tasks, recursing into subtasks."""
    completed = 0
    total = 0
    for item in items:
        subtasks = item.get("subtasks", [])
        if subtasks:
            sub_completed, sub_total = count_tasks(subtasks)
            completed += sub_completed
            total += sub_total
        else:
            total += 1
            if is_complete(item):
                completed += 1
    return completed, total

def find_task_by_id(items, target_id):
    """Recursively find a task by ID."""
    for item in items:
        if item.get("id") == target_id:
            return item
        subtasks = item.get("subtasks", [])
        if subtasks:
            found = find_task_by_id(subtasks, target_id)
            if found:
                return found
    return None

def find_next_incomplete(items):
    """Find first incomplete task (depth-first through subtasks)."""
    for item in items:
        subtasks = item.get("subtasks", [])
        if subtasks:
            found = find_next_incomplete(subtasks)
            if found:
                return found
        elif not is_complete(item) and not is_blocked(item):
            return item
    return None

def get_recent_session_summary():
    """
    Get the most recent pre-compact session summary.

    Returns the content of the most recent snapshot, or None if not found.
    """
    snapshot_dir = Path(".agent/sessions/snapshots")
    if not snapshot_dir.exists():
        return None

    # Find the most recent pre-compact snapshot
    snapshots = list(snapshot_dir.glob("pre-compact-*.md"))
    if not snapshots:
        return None

    # Sort by name (lexicographic = chronological for our timestamp format)
    snapshots.sort(key=lambda p: p.name, reverse=True)
    latest = snapshots[0]

    try:
        content = latest.read_text()
        # Truncate if too long
        if len(content) > MAX_SESSION_SUMMARY_CHARS:
            content = content[:MAX_SESSION_SUMMARY_CHARS]
            # Find a clean break point
            last_newline = content.rfind("\n")
            if last_newline > MAX_SESSION_SUMMARY_CHARS // 2:
                content = content[:last_newline]
            content += "\n[...truncated]"
        return content
    except Exception:
        return None


def compile_context():
    """
    Compile fresh context from memory layers.

    Cache-stable: No timestamps or random values at the top.
    Size-capped: Uses priority-based truncation to stay under MAX_CONTEXT_CHARS.
    """
    sections = []

    # Header (highest priority - always keep)
    sections.append(build_section("header", "# Project Context"))

    # Current task from feature_list.json (critical)
    feature_file = Path("feature_list.json")
    if feature_file.exists():
        try:
            with open(feature_file) as f:
                data = json.load(f)

            features = data.get("features", [])
            completed, total = count_tasks(features)

            # Check for explicit next_task first
            next_task_id = data.get("next_task")
            feat = None
            if next_task_id:
                feat = find_task_by_id(features, next_task_id)
                if feat and (is_complete(feat) or is_blocked(feat)):
                    feat = None  # Skip if already done/blocked

            # Fall back to finding next incomplete
            if not feat:
                feat = find_next_incomplete(features)

            if feat:
                task_content = f"""## Current Task
Progress: {completed}/{total} features complete
**{feat.get('id')}**: {feat.get('name')}
Description: {feat.get('description', '')[:300]}"""
                if feat.get('task'):
                    task_content += f"\nTask file: {feat.get('task')}"
                sections.append(build_section("current_task", task_content))
        except Exception:
            pass

    # Recent session summary from pre-compact (important for context continuity)
    session_summary = get_recent_session_summary()
    if session_summary:
        sections.append(build_section("session_summary", f"## Recent Session Summary\n{session_summary}"))

    # Active constraints (high priority)
    constraints_dir = Path(".agent/memory/constraints")
    if constraints_dir.exists():
        constraint_files = list(constraints_dir.glob("*.md"))[:MAX_CONSTRAINTS]
        if constraint_files:
            constraint_parts = ["## Constraints"]
            for c in constraint_files:
                try:
                    content = c.read_text()[:CONSTRAINT_CHAR_LIMIT]
                    constraint_parts.append(content.strip())
                except Exception:
                    pass
            if len(constraint_parts) > 1:
                sections.append(build_section("constraints", "\n".join(constraint_parts)))

    # Recent failures (important - don't repeat mistakes)
    failures_dir = Path(".agent/memory/failures")
    if failures_dir.exists():
        failure_files = sorted(
            failures_dir.glob("*.md"),
            key=lambda x: x.stat().st_mtime,
            reverse=True
        )[:MAX_FAILURES]

        if failure_files:
            failure_parts = ["## Known Failures (Don't Repeat)"]
            for f in failure_files:
                try:
                    content = f.read_text()[:FAILURE_CHAR_LIMIT]
                    failure_parts.append(content.strip())
                except Exception:
                    pass
            if len(failure_parts) > 1:
                sections.append(build_section("failures", "\n".join(failure_parts)))

    # Working strategies (helpful but expendable)
    strategies_dir = Path(".agent/memory/strategies")
    if strategies_dir.exists():
        strategy_files = sorted(
            strategies_dir.glob("*.md"),
            key=lambda x: x.stat().st_mtime,
            reverse=True
        )[:MAX_STRATEGIES]

        if strategy_files:
            strategy_parts = ["## Working Strategies"]
            for s in strategy_files:
                try:
                    content = s.read_text()[:STRATEGY_CHAR_LIMIT]
                    strategy_parts.append(content.strip())
                except Exception:
                    pass
            if len(strategy_parts) > 1:
                sections.append(build_section("strategies", "\n".join(strategy_parts)))

    # Quick reference with prominent failure recording
    commands_content = """## Commands
**If something fails, record it:**
`.agent/commands.sh failure <id> "what went wrong"`
Example: `.agent/commands.sh failure feat-01 "API returns 401 - auth token not refreshed"`

Other commands:
- `.agent/commands.sh success <id> <msg>` - Mark feature complete
- `.agent/commands.sh recall failures` - See what NOT to do"""
    sections.append(build_section("commands", commands_content))

    # Apply priority-based truncation
    return truncate_by_priority(sections, MAX_CONTEXT_CHARS)

def main():
    """
    Main entry point for SessionStart hook.

    Reads input from Claude Code via stdin, outputs JSON to stdout.
    Errors go to stderr (shown to user but don't block).
    """
    try:
        # Read input from Claude Code
        input_data = json.load(sys.stdin)

        source = input_data.get("source", "unknown")  # startup, resume, or clear

        # Bootstrap .agent/ from template if needed (new worktree)
        bootstrapped = bootstrap_agent_directory()
        if bootstrapped:
            print("🔧 Initialized .agent/ from template", file=sys.stderr)

        # Compile context (no source-specific prefixes for cache stability)
        context = compile_context()

        # Output in format Claude Code expects
        output = {
            "hookSpecificOutput": {
                "hookEventName": "SessionStart",
                "additionalContext": context
            }
        }

        print(json.dumps(output))

        # Info to stderr (optional, shown to user)
        tokens = estimate_tokens(context)
        print(f"📋 Context injected (~{tokens} tokens, {len(context)} chars)", file=sys.stderr)

    except Exception as e:
        # Don't crash - just output empty and log error
        print(json.dumps({}))
        print(f"⚠️ SessionStart hook error: {e}", file=sys.stderr)

if __name__ == "__main__":
    main()
