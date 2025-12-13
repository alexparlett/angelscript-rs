#!/bin/bash
# Quality gates with context-aware checks

echo "🔍 Running Quality Gates..."

PASS=true

# Standard checks
echo -n "  Tests: "
cargo test 2>/dev/null || npm test 2>/dev/null || pytest 2>/dev/null || go test ./... 2>/dev/null && echo "✅" || { echo "❌"; PASS=false; }

echo -n "  Lint: "
cargo clippy 2>/dev/null || npm run lint 2>/dev/null || flake8 . 2>/dev/null || go vet ./... 2>/dev/null && echo "✅" || echo "⚠️"

echo -n "  Build: "
cargo build 2>/dev/null || npm run build 2>/dev/null || go build ./... 2>/dev/null && echo "✅" || { echo "❌"; PASS=false; }

# Context-specific checks
echo -n "  Working context size: "
if [ -f ".agent/working-context/current.md" ]; then
    TOKENS=$(wc -w .agent/working-context/current.md | awk '{print int($1 * 1.3)}')
    if [ "$TOKENS" -gt 8000 ]; then
        echo "⚠️ Large ($TOKENS tokens) - consider compaction"
    else
        echo "✅ ($TOKENS tokens)"
    fi
else
    echo "⚠️ Not compiled"
fi

echo -n "  Failures captured: "
FAILURES=$(ls .agent/memory/failures/*.md 2>/dev/null | wc -l)
echo "$FAILURES recorded"

echo -n "  Strategies captured: "
STRATEGIES=$(ls .agent/memory/strategies/*.md 2>/dev/null | wc -l)
echo "$STRATEGIES recorded"

if [ "$PASS" = true ]; then
    echo ""
    echo "✅ Quality gates passed"
    exit 0
else
    echo ""
    echo "❌ Quality gates failed"
    exit 1
fi
