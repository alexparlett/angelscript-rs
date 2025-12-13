#!/bin/bash
# Memory Manager - Store and retrieve knowledge
# Implements: Retrieval over pinning, schema-driven storage

ACTION="$1"
CATEGORY="$2"  # strategies, constraints, failures, entities
CONTENT="$3"

MEMORY_DIR=".agent/memory"

case "$ACTION" in
    store)
        # Store with metadata for retrieval
        FILENAME="$MEMORY_DIR/$CATEGORY/$(date +%Y%m%d-%H%M%S).md"
        mkdir -p "$MEMORY_DIR/$CATEGORY"
        
        cat > "$FILENAME" << MEMORY
---
created: $(date -Iseconds)
category: $CATEGORY
---
$CONTENT
MEMORY
        echo "🧠 Stored to memory: $FILENAME"
        ;;
        
    retrieve)
        # Retrieve relevant items (not all!)
        echo "🔍 Retrieving from $CATEGORY..."
        if [ -d "$MEMORY_DIR/$CATEGORY" ]; then
            # Simple recency-based retrieval
            # In production, use embeddings or structured queries
            ls -t "$MEMORY_DIR/$CATEGORY"/*.md 2>/dev/null | head -5 | while read f; do
                echo "--- $f ---"
                cat "$f"
                echo ""
            done
        else
            echo "No items in $CATEGORY"
        fi
        ;;
        
    search)
        # Search across memory
        QUERY="$CONTENT"
        echo "🔍 Searching memory for: $QUERY"
        grep -rl "$QUERY" "$MEMORY_DIR" 2>/dev/null | while read f; do
            echo "Found in: $f"
        done
        ;;
        
    *)
        echo "Usage: memory-manager.sh [store|retrieve|search] [category] [content]"
        echo "Categories: strategies, constraints, failures, entities"
        ;;
esac
