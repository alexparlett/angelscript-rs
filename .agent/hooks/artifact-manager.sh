#!/bin/bash
# Artifact Manager - Store large objects by reference
# Implements: Offload to filesystem principle

ACTION="$1"
NAME="$2"
CONTENT="$3"

ARTIFACT_DIR=".agent/artifacts"

case "$ACTION" in
    store)
        # Store content to file, return reference
        CATEGORY="${4:-tool-outputs}"
        FILEPATH="$ARTIFACT_DIR/$CATEGORY/$NAME"
        mkdir -p "$ARTIFACT_DIR/$CATEGORY"
        
        if [ -n "$CONTENT" ]; then
            echo "$CONTENT" > "$FILEPATH"
        else
            # Read from stdin
            cat > "$FILEPATH"
        fi
        
        # Return reference (not content!)
        echo "📦 Artifact stored: $FILEPATH"
        echo "Reference: $FILEPATH"
        ;;
        
    fetch)
        # Fetch artifact content
        FILEPATH="$ARTIFACT_DIR/$NAME"
        if [ -f "$FILEPATH" ]; then
            cat "$FILEPATH"
        else
            # Try with category prefix
            find "$ARTIFACT_DIR" -name "$NAME" -type f | head -1 | xargs cat 2>/dev/null || echo "Artifact not found: $NAME"
        fi
        ;;
        
    list)
        # List available artifacts
        echo "📦 Available artifacts:"
        find "$ARTIFACT_DIR" -type f | while read f; do
            SIZE=$(wc -c < "$f" | tr -d ' ')
            echo "  $f ($SIZE bytes)"
        done
        ;;
        
    *)
        echo "Usage: artifact-manager.sh [store|fetch|list] [name] [content]"
        ;;
esac
