#!/usr/bin/env bash
# setup-branch-protection.sh — apply required-status-checks to protected branches
#
# Resolves #184: per-branch protection now declares which CI check contexts must
# pass before a PR can merge. Names are captured from real recent runs so they
# match what GitHub actually reports as commit statuses.
#
# Usage:
#   ./setup-branch-protection.sh --dry-run            # print, don't apply
#   ./setup-branch-protection.sh                      # apply all branches
#   ./setup-branch-protection.sh --branch v3          # apply just v3
#
# Requires: gh CLI authenticated with admin permission on pacphi/sindri,
# and `jq`. Compatible with bash 3.2+ (no associative arrays used).

set -euo pipefail

REPO="${REPO:-pacphi/sindri}"
DRY_RUN=0
ONLY_BRANCH=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) DRY_RUN=1 ;;
    --branch)  ONLY_BRANCH="$2"; shift ;;
    --help|-h)
      sed -n '2,16p' "$0"; exit 0 ;;
    *)
      echo "Unknown arg: $1" >&2; exit 2 ;;
  esac
  shift
done

# Aggregate gate jobs are preferred over enumerating every matrix leaf:
#   - GitHub treats each context name as a literal merge gate
#   - matrix-leaf names include parameterized suffixes that change when the
#     matrix is edited; in-pipeline `needs:` already aggregates them
#
# Captured from real runs as of 2026-04-26.
contexts_for() {
  case "$1" in
    main)
      cat <<'EOF'
Validate YAML
Validate Markdown
Validate Shell
Check Links
EOF
      ;;
    v1)
      cat <<'EOF'
CI / Markdown lint
EOF
      ;;
    v2)
      cat <<'EOF'
CI / CI v2 Required Checks
CI / Build v2 Docker Image
CI / Security Scan
EOF
      ;;
    v3)
      cat <<'EOF'
CI / CI v3 Required Checks
CI / Rust workspace / Workspace guard
CI / Rust workspace / cargo fmt
CI / Rust workspace / cargo clippy
CI / Rust workspace / cargo test
CI / Rust workspace / cargo build
CI / Security Scan
CI / Security Audit
EOF
      ;;
    v4)
      cat <<'EOF'
CI / Build & Test (ubuntu-latest)
CI / Build & Test (ubuntu-24.04-arm)
CI / Build & Test (macos-14)
CI / Build & Test (windows-latest)
CI / Security audit
EOF
      ;;
    *)
      return 1 ;;
  esac
}

apply_branch() {
  local branch="$1"
  local raw
  if ! raw="$(contexts_for "$branch")"; then
    echo "::warning:: no contexts defined for $branch" >&2
    return 0
  fi

  local ctx_json
  ctx_json=$(printf '%s\n' "$raw" | sed '/^$/d' | jq -R . | jq -sc .)

  local body
  body=$(jq -n \
    --argjson contexts "$ctx_json" \
    '{
      required_status_checks: { strict: false, contexts: $contexts },
      enforce_admins: false,
      required_pull_request_reviews: { required_approving_review_count: 1, dismiss_stale_reviews: true },
      restrictions: null,
      allow_force_pushes: false,
      allow_deletions: false
    }')

  echo "── $branch ──"
  echo "$body" | jq .

  if (( DRY_RUN )); then
    echo "(dry-run, not applied)"
    return 0
  fi

  printf '%s' "$body" | gh api -X PUT "repos/${REPO}/branches/${branch}/protection" --input -
  echo "✅ applied to $branch"
}

if [[ -n "$ONLY_BRANCH" ]]; then
  apply_branch "$ONLY_BRANCH"
else
  for b in main v1 v2 v3 v4; do
    apply_branch "$b"
  done
fi
