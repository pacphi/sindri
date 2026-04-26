#!/usr/bin/env bash
# build-branches.sh — materialize v1, v2, v3, v4 branches and reshape main
# from the current chore/repo-reorg HEAD using the manifests in this dir.
#
# IDEMPOTENT: re-running deletes and recreates the v* branches locally.
# DOES NOT PUSH. After running, inspect with `git log --oneline v1 v2 v3 v4`
# and only then push with explicit user approval.
#
# Prerequisites:
#   - Currently on `chore/repo-reorg` with all preparatory commits applied
#   - `research/v4` fetched locally (git fetch origin research/v4:research/v4)
#   - `pre-reorg-2026-04-25` safety tag exists
#
# Usage:
#   ./scripts/reorg/build-branches.sh [--dry-run]

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"
SCRIPT_DIR=scripts/reorg

DRY_RUN=0
[[ "${1:-}" == "--dry-run" ]] && DRY_RUN=1

current_branch=$(git rev-parse --abbrev-ref HEAD)
if [[ "$current_branch" != "chore/repo-reorg" ]]; then
  echo "FATAL: must be on chore/repo-reorg (currently on $current_branch)" >&2
  exit 1
fi

if ! git rev-parse pre-reorg-2026-04-25 >/dev/null 2>&1; then
  echo "FATAL: safety tag pre-reorg-2026-04-25 missing. Run: git tag pre-reorg-2026-04-25 main" >&2
  exit 1
fi

# Only block on tracked-but-uncommitted changes. Untracked files (like a
# local CLAUDE.md that the developer regenerates locally) are fine.
if git status --porcelain | grep -vE '^\?\?' | grep -q .; then
  echo "FATAL: working tree has uncommitted tracked changes. Commit or stash before running." >&2
  git status --short
  exit 1
fi

run() {
  echo "+ $*"
  if [[ $DRY_RUN -eq 0 ]]; then "$@"; fi
}

# Compute paths to delete: tracked paths NOT matched by the manifest.
# Manifest entries can be exact paths or directories (trailing slash treated as recursive).
paths_to_delete_for() {
  local manifest="$1"
  local tmp_keep
  tmp_keep=$(mktemp)
  # Normalize manifest:
  #   1. drop comment-only and blank lines
  #   2. strip inline `# comments` and trailing whitespace
  #   3. strip trailing slashes (we match dirs by prefix)
  grep -vE '^\s*(#|$)' "$manifest" \
    | sed -E 's/[[:space:]]+#.*$//; s/[[:space:]]+$//; s|/$||' \
    | grep -vE '^$' \
    | sort -u > "$tmp_keep"

  git ls-files | while read -r f; do
    keep=0
    while IFS= read -r m; do
      # Exact match
      if [[ "$f" == "$m" ]]; then keep=1; break; fi
      # Directory prefix match (manifest entry is a dir name)
      if [[ "$f" == "$m"/* ]]; then keep=1; break; fi
    done < "$tmp_keep"
    if [[ $keep -eq 0 ]]; then echo "$f"; fi
  done
  rm -f "$tmp_keep"
}

install_ai_templates() {
  local br="$1"
  local tpl="$SCRIPT_DIR/branch-templates/CLAUDE.md.${br}"
  if [[ -f "$tpl" ]]; then
    cp "$tpl" CLAUDE.md
    cp "$tpl" AGENTS.md
    git add -f CLAUDE.md AGENTS.md
  fi
}

# Promote per-version root files (README, CHANGELOG, RELEASE_NOTES) from their
# in-tree location (v*/README.md etc.) up to branch root, replacing the
# umbrella forms that live on chore/repo-reorg. Runs BEFORE manifest deletion
# so the promoted files are present when the cleaning sweep happens.
promote_root_files() {
  local br="$1"
  case "$br" in
    v1)
      cp v1/CHANGELOG.md CHANGELOG.md
      cat > README.md <<'README_EOF'
# Sindri v1 (END OF LIFE)

v1 is the original Bash implementation. It is end-of-life and accepts only
critical security backports. See [`CHANGELOG.md`](CHANGELOG.md) for the
historical record.

For active development, switch to the [`v3`](https://github.com/pacphi/sindri/tree/v3)
or [`v4`](https://github.com/pacphi/sindri/tree/v4) branch.
README_EOF
      git add README.md CHANGELOG.md
      ;;
    v2)
      [[ -f v2/README.md    ]] && cp v2/README.md    README.md
      [[ -f v2/CHANGELOG.md ]] && cp v2/CHANGELOG.md CHANGELOG.md
      [[ -f RELEASE_NOTES.v2.md ]] && git mv RELEASE_NOTES.v2.md RELEASE_NOTES.md
      # Relocate root examples/v2/ → v2/examples/ so they survive the manifest sweep.
      if [[ -d examples/v2 ]]; then
        git mv examples/v2 v2/examples 2>&1 | head -3
      fi
      git add README.md CHANGELOG.md RELEASE_NOTES.md 2>/dev/null || true
      ;;
    v3)
      [[ -f v3/README.md    ]] && cp v3/README.md    README.md
      [[ -f v3/CHANGELOG.md ]] && cp v3/CHANGELOG.md CHANGELOG.md
      [[ -f RELEASE_NOTES.v3.md   ]] && git mv RELEASE_NOTES.v3.md   RELEASE_NOTES.md
      [[ -f RELEASE_NOTES.v3.1.md ]] && git mv RELEASE_NOTES.v3.1.md RELEASE_NOTES.3.1.md
      # Relocate root examples/v3/ → v3/examples/ (v3 already has v3/examples/
      # for crate-level examples; merge into a v3/examples/integration/ subdir
      # if a collision exists, otherwise straight rename).
      if [[ -d examples/v3 ]]; then
        if [[ -d v3/examples ]]; then
          git mv examples/v3 v3/examples-integration 2>&1 | head -3
        else
          git mv examples/v3 v3/examples 2>&1 | head -3
        fi
      fi
      git add README.md CHANGELOG.md RELEASE_NOTES.md RELEASE_NOTES.3.1.md 2>/dev/null || true
      ;;
    v4)
      if [[ -f v4/README.md ]]; then
        cp v4/README.md README.md
      else
        cat > README.md <<'README_EOF'
# Sindri v4

v4 is the next-generation Rust implementation, promoted from `research/v4`
during the April 2026 reorg. See [`v4/`](v4/) for source and `v4/docs/` for
architecture (ADRs, DDDs, plan).
README_EOF
      fi
      cat > CHANGELOG.md <<'CHANGELOG_EOF'
# Changelog — v4

All notable changes to v4 are recorded here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and adheres to
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- v4 promoted from the `research/v4` branch on 2026-04-25 as part of the repo
  reorganization (see `pre-reorg-2026-04-25` tag and `research-v4-final` tag).
CHANGELOG_EOF
      git add README.md CHANGELOG.md
      ;;
    main)
      : # main keeps its umbrella README and CHANGELOG already on chore/repo-reorg
      ;;
  esac
}

build_branch() {
  local br="$1"
  local manifest="$SCRIPT_DIR/manifest-${br}.txt"
  if [[ ! -f "$manifest" ]]; then
    echo "FATAL: $manifest missing" >&2
    exit 1
  fi

  echo "── Building $br ───────────────────────────────────────────────"
  run git checkout -B "$br" chore/repo-reorg

  if [[ $DRY_RUN -eq 0 ]]; then
    promote_root_files "$br"
    install_ai_templates "$br"
  fi

  local del_list
  del_list=$(mktemp)
  paths_to_delete_for "$manifest" > "$del_list"
  local count
  count=$(wc -l < "$del_list" | tr -d ' ')
  echo "  deletions: $count paths"

  if [[ $count -gt 0 ]]; then
    if [[ $DRY_RUN -eq 0 ]]; then
      tr '\n' '\0' < "$del_list" | xargs -0 git rm -rf --quiet --
    else
      head -20 "$del_list" | sed 's/^/    rm /'
      [[ $count -gt 20 ]] && echo "    ... and $((count - 20)) more"
    fi
  fi
  rm -f "$del_list"

  if [[ $DRY_RUN -eq 0 ]]; then
    # --no-verify: husky lint-staged depends on package.json which v1/v2/v4 by
    # design no longer have. The hook is fundamentally incompatible with this
    # structural reorg commit. Justified single-purpose bypass.
    git commit --quiet --no-verify -m "chore($br): isolate $br tree from monorepo

Removes paths not listed in scripts/reorg/manifest-${br}.txt as part of the
repo reorganization (see docs/REPO_REORG_PLAN.md). History is preserved;
SHAs are not rewritten."
  fi
  run git checkout chore/repo-reorg
}

# Phase A: pre-stage moves on chore/repo-reorg (caller responsibility).
# At this point, examples/v2 should already have been git-mv'd to v2/examples,
# RELEASE_NOTES.v2.md to v2/RELEASE_NOTES.md, etc. — done by separate
# preparatory commits before invoking this script. See README.md.

# Phase B: build the four sibling branches.
for br in v1 v2 v3 v4; do
  build_branch "$br"
done

# Phase C: reshape main.
echo "── Reshaping main ─────────────────────────────────────────────"
run git checkout -B main-reorg-staging chore/repo-reorg
del_list=$(mktemp)
paths_to_delete_for "$SCRIPT_DIR/manifest-main.txt" > "$del_list"
count=$(wc -l < "$del_list" | tr -d ' ')
echo "  deletions: $count paths"
if [[ $count -gt 0 && $DRY_RUN -eq 0 ]]; then
  tr '\n' '\0' < "$del_list" | xargs -0 git rm -r --quiet --
  git commit --quiet --no-verify -m "chore(main): remove product source after sibling-branch creation

After v1/v2/v3/v4 branches were created (see chore/repo-reorg history),
main no longer carries product source. It now hosts only umbrella docs,
repo-wide governance, and centralized .github/ that routes to v* branches."
fi
rm -f "$del_list"

echo
echo "✅ Local materialization complete."
echo "Inspect:"
echo "    git log --oneline -5 v1 v2 v3 v4 main-reorg-staging"
echo "    git ls-tree --name-only v3 | head"
echo
echo "When verified, the cutover commands (run by a human) are:"
echo "    git checkout main && git merge --ff-only main-reorg-staging"
echo "    git push origin main v1 v2 v3 v4 pre-reorg-2026-04-25"
echo "    git push origin :research/v4   # only after tagging it: git tag research-v4-final research/v4 && git push origin research-v4-final"
