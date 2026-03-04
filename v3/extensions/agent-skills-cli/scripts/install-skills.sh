#!/usr/bin/env bash
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_FILE="/tmp/agent-skills-install.log"
SKILLS_DIR="$HOME/.claude/skills"

SUCCESS=0
FAIL=0
FAILED=()

print_status()  { echo "  $*"; }
print_warning() { echo "  WARNING: $*"; }
print_success() { echo "  $*"; }

print_status "Installing skills via agent-skills-cli..."
echo "=== agent-skills-cli skill install ===" > "$LOG_FILE"
echo "Started: $(date)" >> "$LOG_FILE"

mkdir -p "$SKILLS_DIR"

# Ensure skills shim is available
if ! command -v skills &>/dev/null; then
  mise reshim 2>/dev/null || true
fi
SKILLS_BIN="$(command -v skills 2>/dev/null || echo "")"

# Git sparse-checkout fallback — copies the entire subdir (preserving scripts/assets)
# Usage: git_sparse_install <name> <repo_url> <branch> <subpath>
git_sparse_install() {
  local name="$1" repo_url="$2" branch="$3" subpath="$4"
  local dest="$SKILLS_DIR/$name"
  local tmp
  tmp=$(mktemp -d)
  # shellcheck disable=SC2064
  trap "rm -rf '$tmp'" RETURN

  git -C "$tmp" init -q 2>>"$LOG_FILE" &&
  git -C "$tmp" remote add origin "$repo_url" 2>>"$LOG_FILE" &&
  git -C "$tmp" sparse-checkout set "$subpath" 2>>"$LOG_FILE" &&
  git -C "$tmp" pull --depth 1 origin "$branch" -q 2>>"$LOG_FILE" &&
  cp -r "$tmp/$subpath" "$dest" 2>>"$LOG_FILE"
}

# ── Stage 1: Install all awesome-claude-skills in one clone ──
AWESOME_SKILLS=(
  brand-guidelines canvas-design competitive-ads-extractor content-research-writer
  domain-name-brainstormer file-organizer internal-comms invoice-organizer
  lead-research-assistant mcp-builder meeting-insights-analyzer raffle-winner-picker
  skill-creator slack-gif-creator template-skill theme-factory
  video-downloader webapp-testing artifacts-builder
)
AWESOME_REPO="https://github.com/ComposioHQ/awesome-claude-skills"
AWESOME_BRANCH="master"

print_status "Installing ${#AWESOME_SKILLS[@]} skills from awesome-claude-skills..."

AWESOME_OK=false
if [ -n "$SKILLS_BIN" ]; then
  echo "[skills CLI] Installing awesome-claude-skills batch" >> "$LOG_FILE"
  if "$SKILLS_BIN" install "$AWESOME_REPO" \
      --agent claude --global --yes \
      --skill "${AWESOME_SKILLS[@]}" >> "$LOG_FILE" 2>&1; then
    AWESOME_OK=true
    ((SUCCESS += ${#AWESOME_SKILLS[@]}))
    print_status "  Batch install: OK (${#AWESOME_SKILLS[@]} skills)"
  fi
fi

# Stage 2 fallback: install any missing skills individually via sparse-checkout
if [ "$AWESOME_OK" = false ]; then
  print_status "  Falling back to git sparse-checkout per skill..."
  for NAME in "${AWESOME_SKILLS[@]}"; do
    printf "  %-35s" "$NAME"
    if git_sparse_install "$NAME" "${AWESOME_REPO}.git" "$AWESOME_BRANCH" "$NAME"; then
      echo "OK"
      ((SUCCESS++))
    else
      echo "FAILED"
      ((FAIL++))
      FAILED+=("$NAME")
    fi
  done
fi

# ── Install prompt-improver (single-plugin repo) ──
PROMPT_REPO="https://github.com/severity1/claude-code-prompt-improver"
printf "  %-35s" "prompt-improver"
INSTALLED=false

if [ -n "$SKILLS_BIN" ]; then
  if "$SKILLS_BIN" install "$PROMPT_REPO" \
      --agent claude --global --yes >> "$LOG_FILE" 2>&1; then
    INSTALLED=true
  fi
fi

if [ "$INSTALLED" = false ]; then
  TMP=$(mktemp -d)
  if git clone --depth 1 -q "$PROMPT_REPO" "$TMP/repo" 2>>"$LOG_FILE" &&
     cp -r "$TMP/repo" "$SKILLS_DIR/prompt-improver" 2>>"$LOG_FILE"; then
    INSTALLED=true
  fi
  rm -rf "$TMP"
fi

if [ "$INSTALLED" = true ]; then
  echo "OK"
  ((SUCCESS++))
else
  echo "FAILED"
  ((FAIL++))
  FAILED+=("prompt-improver")
fi

# ── Summary ──
TOTAL=$((${#AWESOME_SKILLS[@]} + 1))
echo ""
print_status "Result: $SUCCESS/$TOTAL skills installed"
[ ${#FAILED[@]} -gt 0 ] && print_warning "Failed (${FAIL}): ${FAILED[*]}" && print_status "See $LOG_FILE"
print_success "Skills available at: $SKILLS_DIR"
exit 0
