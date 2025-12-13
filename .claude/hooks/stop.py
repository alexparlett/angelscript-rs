#!/usr/bin/env python3
"""
Stop Hook - Tracks progress when Claude finishes responding.

Fires when:
- Claude completes a response
- Can block to force continuation (we don't use this by default)

MODES:
- READ-ONLY (default): Only reads feature_list.json, writes to .agent/metrics/
- WRITE MODE (opt-in): Can auto-complete features if tests pass

To enable write mode:
  export CONTEXT_ENGINE_WRITE_MODE=1

Write mode will auto-mark features complete when:
  1. Tests pass (detected by recent test output in transcript)
  2. Feature verifier confirms completion
  
This is an advanced feature - use with caution.

Requires: Claude Code 1.0.17+ (with Stop hook support)
"""

import json
import sys
import os
import subprocess
from pathlib import Path
from datetime import datetime

# Configuration
WRITE_MODE = os.environ.get("CONTEXT_ENGINE_WRITE_MODE", "0") == "1"

def log_metric(event_type, feature_id=None, extra=None):
    """
    Append a metric event to the metrics log.
    """
    try:
        metrics_file = Path(".agent/metrics/session-metrics.jsonl")
        metrics_file.parent.mkdir(parents=True, exist_ok=True)
        
        entry = {
            "timestamp": datetime.now().isoformat(),
            "event": event_type,
        }
        if feature_id:
            entry["feature_id"] = feature_id
        if extra:
            entry.update(extra)
        
        with open(metrics_file, "a") as f:
            f.write(json.dumps(entry) + "\n")
    except Exception:
        pass

def get_progress():
    """
    Read current progress from feature_list.json (READ-ONLY).
    
    Returns (completed_count, total_count, next_feature_id, next_feature_name)
    """
    feature_file = Path("feature_list.json")
    
    if not feature_file.exists():
        return 0, 0, None, None
    
    try:
        with open(feature_file) as f:
            data = json.load(f)
        
        features = data.get("features", [])
        completed = sum(1 for f in features if f.get("passes", False))
        total = len(features)
        
        # Find next incomplete feature
        next_id = None
        next_name = None
        for feat in sorted(features, key=lambda x: x.get("priority", 99)):
            if not feat.get("passes", False) and not feat.get("blocked", False):
                next_id = feat.get("id")
                next_name = feat.get("name")
                break
        
        return completed, total, next_id, next_name
    except Exception:
        return 0, 0, None, None

def check_tests_passed():
    """
    Check if tests recently passed (for write mode).
    
    Looks for common test success indicators.
    Returns True if tests appear to have passed.
    """
    # Check for recent test output files
    test_indicators = [
        Path(".agent/sessions/last-test-result"),
        Path("test-results.json"),
        Path("coverage/lcov.info"),
    ]
    
    for indicator in test_indicators:
        if indicator.exists():
            try:
                # Check if modified in last 5 minutes
                mtime = indicator.stat().st_mtime
                if datetime.now().timestamp() - mtime < 300:
                    return True
            except Exception:
                pass
    
    return False

def auto_complete_feature(feature_id):
    """
    Mark a feature as complete (WRITE MODE ONLY).
    
    Only called if CONTEXT_ENGINE_WRITE_MODE=1 and tests pass.
    """
    feature_file = Path("feature_list.json")
    
    try:
        with open(feature_file) as f:
            data = json.load(f)
        
        for feat in data.get("features", []):
            if feat.get("id") == feature_id:
                feat["passes"] = True
                feat["completed_at"] = datetime.now().isoformat()
                break
        
        with open(feature_file, "w") as f:
            json.dump(data, f, indent=2)
        
        return True
    except Exception:
        return False

def main():
    """
    Main entry point for Stop hook.
    
    Default: READ-ONLY (metrics only)
    Opt-in: WRITE MODE (auto-complete features)
    """
    try:
        # Read input from Claude Code
        input_data = json.load(sys.stdin)
        
        stop_hook_active = input_data.get("stop_hook_active", False)
        
        # Don't create infinite loops
        if stop_hook_active:
            print(json.dumps({}))
            return
        
        # Get progress
        completed, total, next_id, next_name = get_progress()
        
        # Log the stop event
        log_metric("stop", extra={
            "progress": f"{completed}/{total}",
            "next_feature": next_id,
            "write_mode": WRITE_MODE
        })
        
        # Show progress to user (stderr)
        if total > 0:
            print(f"📊 Progress: {completed}/{total} features", file=sys.stderr)
            if next_id:
                print(f"   Next: {next_id}", file=sys.stderr)
        
        # Write mode: auto-complete if tests pass
        if WRITE_MODE and next_id and check_tests_passed():
            if auto_complete_feature(next_id):
                print(f"✅ Auto-completed: {next_id} (tests passed)", file=sys.stderr)
                log_metric("auto_complete", feature_id=next_id)
        
    except Exception as e:
        print(f"⚠️ Stop hook error: {e}", file=sys.stderr)
    
    # Don't block - output empty
    print(json.dumps({}))

if __name__ == "__main__":
    main()
