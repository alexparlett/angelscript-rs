#!/bin/bash
# Schema-Driven Summarizer
# Implements: Preservation schema for critical information

INPUT="$1"
OUTPUT="${2:-.agent/working-context/summary.md}"

# Summarization schema - what MUST survive
SCHEMA_PROMPT='Summarize the following while PRESERVING these required elements:

## Required Preservation Schema:
1. **Causal Steps**: Chain of decisions and WHY they were made
2. **Active Constraints**: Rules and requirements still in effect
3. **Failures**: What was tried and did NOT work (prevent repetition)
4. **Open Commitments**: Promises or tasks not yet fulfilled
5. **Key Entities**: Names, IDs, paths that must stay resolvable

## Rules:
- Preserve decision-relevant structure
- Keep specific identifiers (file paths, function names, error codes)
- Maintain causal relationships
- Do NOT compress away constraints
- Do NOT lose failure information

## Content to Summarize:
'

if [ -f "$INPUT" ]; then
    echo "📋 Summarizing with preservation schema..."
    echo "$SCHEMA_PROMPT" > "$OUTPUT.prompt"
    cat "$INPUT" >> "$OUTPUT.prompt"
    echo ""
    echo "Schema prompt written to: $OUTPUT.prompt"
    echo "Run through LLM to generate summary, then save to: $OUTPUT"
else
    echo "Usage: summarize.sh [input-file] [output-file]"
    echo ""
    echo "Preservation schema ensures these survive:"
    echo "  - Causal steps (decisions and why)"
    echo "  - Active constraints"
    echo "  - Failures (what didn't work)"
    echo "  - Open commitments"
    echo "  - Key entities"
fi
