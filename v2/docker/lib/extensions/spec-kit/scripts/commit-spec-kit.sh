#!/usr/bin/env bash
set -euo pipefail

# Commit spec-kit initialization files if there are changes
if [[ -n "$(git status --porcelain 2>/dev/null)" ]]; then
    git add . 2>/dev/null || true
    git commit -m "feat: add GitHub spec-kit configuration" 2>/dev/null || true
    echo "spec-kit files committed"
else
    echo "No changes to commit"
fi
