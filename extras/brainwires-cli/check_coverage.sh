#!/bin/bash

echo "=== COVERAGE BY MODULE ==="
echo ""

for dir in src/agents src/auth src/commands src/config src/mcp src/mcp_server src/providers src/storage src/tools src/types src/utils src/tui src/cli; do
    if [ -d "$dir" ]; then
        total=$(find "$dir" -name "*.rs" -type f | wc -l)
        tested=$(find "$dir" -name "*.rs" -type f -exec grep -l "#\[cfg(test)\]" {} \; 2>/dev/null | wc -l)
        echo "$dir: $tested/$total files have tests"
    fi
done
