#!/usr/bin/env bash
# audit.sh — classify every tracked top-level path by version-ownership.
# Output: scripts/reorg/AUDIT.txt
#
# Classification rules (see docs/REPO_REORG_PLAN.md §4):
#   v1     — v1/**
#   v2     — v2/**, examples/v2/**, RELEASE_NOTES.v2.md, .github/ISSUE_TEMPLATE/bug_report_v2.md
#   v3     — v3/**, examples/v3/**, packages/@pacphi/**, RELEASE_NOTES.v3*.md,
#            package.json, pnpm-workspace.yaml, .npmrc, .research/**,
#            .github/ISSUE_TEMPLATE/bug_report_v3.md
#   v4     — (sourced separately from research/v4 branch; not present on main)
#   main   — LICENSE, README.md, CHANGELOG.md, sindri.png, docs/** (governance only),
#            .github/{CODEOWNERS,dependabot.yml,workflows,actions,scripts,docs,templates,ISSUE_TEMPLATE/{config.yml,documentation.md,extension_request.md,feature_request.md},WORKFLOW_ARCHITECTURE.md},
#            .markdownlint*, .prettier*, .shellcheckrc, .yamllint.yml, .gitignore,
#            .dockerignore, .env.example, .husky/**, .claude/** (shared agents/skills),
#            scripts/reorg/** (this tooling), Makefile (split later)
#   purge  — none currently tracked (* 2 / * 3 dups are all untracked)
#
# This script is read-only.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

OUT=scripts/reorg/AUDIT.txt

classify() {
  local p="$1"
  case "$p" in
    v1|v1/*)                                 echo v1 ;;
    v2|v2/*|examples/v2/*|RELEASE_NOTES.v2.md) echo v2 ;;
    .github/ISSUE_TEMPLATE/bug_report_v2.md) echo v2 ;;
    v3|v3/*|examples/v3/*|packages/*|RELEASE_NOTES.v3*.md|package.json|pnpm-workspace.yaml|.npmrc|.research|.research/*) echo v3 ;;
    .github/ISSUE_TEMPLATE/bug_report_v3.md) echo v3 ;;
    v4|v4/*)                                 echo v4 ;;
    *)                                       echo main ;;
  esac
}

{
  echo "# Sindri Repo Audit — generated $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "# Branch: $(git rev-parse --abbrev-ref HEAD)"
  echo "# Commit: $(git rev-parse HEAD)"
  echo
  echo "## Tracked top-level entries"
  git ls-files | awk -F/ '{print $1}' | sort -u | while read -r top; do
    count=$(git ls-files -- "$top" | wc -l | tr -d ' ')
    klass=$(classify "$top")
    printf "  %-8s %5d  %s\n" "$klass" "$count" "$top"
  done
  echo
  echo "## Per-classification file totals"
  git ls-files | while read -r f; do
    echo "$(classify "$f")"
  done | sort | uniq -c | sort -rn | sed 's/^/  /'
  echo
  echo "## Untracked stray dups (filesystem only)"
  find . -maxdepth 8 \( -name "* 2" -o -name "* 3" \) \
    -not -path "./node_modules/*" \
    -not -path "./.git/*" \
    -not -path "./v3/target/*" \
    2>/dev/null | sed 's|^\./|  |' | sort
} > "$OUT"

echo "Wrote $OUT ($(wc -l < "$OUT") lines)"
