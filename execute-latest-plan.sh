#!/bin/bash


# PROMPT="use sub agent development and git worktree to execute the latest plan, Keep the branch as-is (I will handle it later)"
# claude --permission-mode bypassPermissions --model sonnet --effort high "$PROMPT"

set -euo pipefail

PLANS_DIR="docs/superpowers/plans"

# Newest plan by date prefix (YYYY-MM-DD from filename), with mtime as
# tiebreaker for same-day plans. Strategy: emit "<date> <mtime-rank> <path>",
# sort descending on date then ascending on mtime-rank (ls -t already sorted
# newest-first, so rank 0 = newest mtime), take first.
PLAN_FILE=$(ls -t "$PLANS_DIR"/*.md 2>/dev/null \
    | awk '{
        n = split($0, parts, "/");
        basename = parts[n];
        date = substr(basename, 1, 10);
        printf "%s %04d %s\n", date, NR-1, $0
    }' \
    | sort -k1,1r -k2,2n \
    | awk 'NR==1{print $3}') || true

if [ -z "$PLAN_FILE" ]; then
    echo "Error: no plan files found in $PLANS_DIR" >&2
    exit 1
fi

PROMPT=$(printf '%s' \
    "/subagent-driven-development " \
    "@${PLAN_FILE}"
)

echo "$PROMPT"

read -r -n 1 -s -p "Press any key to continue..."

claude --permission-mode bypassPermissions "$PROMPT"

# PROMPT="use sub agent development and git worktree to execute the latest plan, Keep the branch as-is (I will handle it later)"
# claude --permission-mode bypassPermissions --model sonnet --effort high "$PROMPT"