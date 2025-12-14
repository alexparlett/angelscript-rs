#!/usr/bin/env python3
"""
PostToolUse Hook - Logs tool usage after Claude uses a tool.

Fires when:
- Claude uses Write, Edit, MultiEdit, Bash, etc.

IMPORTANT: This hook is READ-ONLY by default. It only logs to
.agent/sessions/activity.jsonl. It does NOT run linters, formatters,
or any external commands unless users explicitly enable them.

To enable linting (optional):
  Set CONTEXT_ENGINE_LINT=1 environment variable

Requires: Claude Code 1.0.17+ (with PostToolUse hook support)
"""

import json
import sys
import os
from pathlib import Path
from datetime import datetime

# Optional linting (disabled by default for safety)
ENABLE_LINTING = os.environ.get("CONTEXT_ENGINE_LINT", "0") == "1"

def log_activity(tool_name, tool_input):
    """
    Log tool usage to activity log (READ-ONLY operation).
    """
    try:
        log_file = Path(".agent/sessions/activity.jsonl")
        log_file.parent.mkdir(parents=True, exist_ok=True)
        
        # Extract relevant info without storing full content
        summary = {}
        if tool_name in ("Write", "Edit", "MultiEdit"):
            summary["file"] = tool_input.get("file_path", "unknown")
        elif tool_name == "Bash":
            cmd = tool_input.get("command", "")
            summary["command"] = cmd[:100]  # Truncate long commands
        
        entry = {
            "timestamp": datetime.now().isoformat(),
            "tool": tool_name,
            "summary": summary
        }
        
        with open(log_file, "a") as f:
            f.write(json.dumps(entry) + "\n")
    except Exception:
        pass  # Don't crash on logging failure

def main():
    """
    Main entry point for PostToolUse hook.
    
    READ-ONLY by default. Only logs activity.
    """
    try:
        # Read input from Claude Code
        input_data = json.load(sys.stdin)
        
        tool_name = input_data.get("tool_name", "")
        tool_input = input_data.get("tool_input", {})
        
        # Log activity (always)
        log_activity(tool_name, tool_input)
        
        # Optional linting (only if explicitly enabled)
        # if ENABLE_LINTING and tool_name in ("Write", "Edit", "MultiEdit"):
        #     file_path = tool_input.get("file_path", "")
        #     # Run linter here if enabled
        #     pass
        
    except Exception as e:
        print(f"⚠️ PostToolUse hook error: {e}", file=sys.stderr)
    
    # Output empty - no blocking
    print(json.dumps({}))

if __name__ == "__main__":
    main()
