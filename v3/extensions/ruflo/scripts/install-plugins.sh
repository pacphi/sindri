#!/usr/bin/env bash
set -euo pipefail

# Ruflo Plugin Auto-Installation Script
# Purpose: Install 15 core plugins after extension installation
# Exit codes: 0 (success, even with partial failures), 1 (critical failure)

LOG_DIR="${SINDRI_LOG_DIR:-/tmp}"
mkdir -p "$LOG_DIR"
LOG_FILE="${LOG_DIR}/plugin-install.log"
PREFIX="[ruflo]"

# Plugin list (12 working + 3 broken upstream)
PLUGINS=(
  "@claude-flow/plugin-gastown-bridge"
  "@claude-flow/plugin-prime-radiant"
  "@claude-flow/plugin-test-intelligence"
  "@claude-flow/plugin-perf-optimizer"
  "@claude-flow/plugin-neural-coordination"
  "@claude-flow/plugin-cognitive-kernel"
  "@claude-flow/plugin-quantum-optimizer"
  "@claude-flow/teammate-plugin"
  "@claude-flow/embeddings"
  "@claude-flow/claims"
  "@claude-flow/plugins"
  "@claude-flow/performance"
  # --- Broken upstream deps (uncomment when fixed on npm) ---
  # "@claude-flow/plugin-code-intelligence"  # E404: depends on unpublished @claude-flow/ruvector-upstream
  # "@claude-flow/security"                  # ETARGET: peer dep needs @claude-flow/security@>=3.0.0, only alpha on npm
  # "@claude-flow/neural"                    # ETARGET: peer dep needs @claude-flow/memory@>=3.0.0, only alpha on npm
)

# Counters
SUCCESS_COUNT=0
FAIL_COUNT=0
FAILED_PLUGINS=()

# Initialize log file
echo "=== Ruflo Plugin Installation ===" > "$LOG_FILE"
echo "Started: $(date)" >> "$LOG_FILE"
echo "" >> "$LOG_FILE"

# Step 1: Verify ruflo installation (CRITICAL)
echo "$PREFIX Verifying installation..." | tee -a "$LOG_FILE"

if ! ruflo --version | grep -q 'v[0-9]'; then
  echo "✗ Ruflo not found or wrong version" | tee -a "$LOG_FILE"
  echo "Error: ruflo must be installed first" | tee -a "$LOG_FILE"
  exit 1
fi

echo "✓ Ruflo installed successfully" | tee -a "$LOG_FILE"
echo "" | tee -a "$LOG_FILE"

# Step 2: Install plugins (NON-CRITICAL)
echo "$PREFIX Installing ${#PLUGINS[@]} core plugins..." | tee -a "$LOG_FILE"

for plugin in "${PLUGINS[@]}"; do
  echo "  Installing $plugin..." | tee -a "$LOG_FILE"

  if npx ruflo@latest plugins install --name "$plugin" >> "$LOG_FILE" 2>&1; then
    echo "  ✓ $plugin installed" | tee -a "$LOG_FILE"
    ((++SUCCESS_COUNT))
  else
    echo "  ✗ $plugin failed" | tee -a "$LOG_FILE"
    ((++FAIL_COUNT))
    FAILED_PLUGINS+=("$plugin")
  fi
done

echo "" | tee -a "$LOG_FILE"

# Step 3: Report summary
echo "$PREFIX Plugin installation complete:" | tee -a "$LOG_FILE"
echo "  ✓ Installed: $SUCCESS_COUNT/${#PLUGINS[@]}" | tee -a "$LOG_FILE"

if [ $FAIL_COUNT -gt 0 ]; then
  echo "  ✗ Failed: $FAIL_COUNT/${#PLUGINS[@]}" | tee -a "$LOG_FILE"
  echo "" | tee -a "$LOG_FILE"
  echo "Failed plugins (can be installed manually):" | tee -a "$LOG_FILE"
  for failed_plugin in "${FAILED_PLUGINS[@]}"; do
    echo "  - $failed_plugin" | tee -a "$LOG_FILE"
  done
  echo "" | tee -a "$LOG_FILE"
  echo "To retry failed plugins, run:" | tee -a "$LOG_FILE"
  echo "  npx ruflo@latest plugins install --name <plugin-name>" | tee -a "$LOG_FILE"
fi

echo "" | tee -a "$LOG_FILE"
echo "Completed: $(date)" >> "$LOG_FILE"
echo "Full log: $LOG_FILE" | tee -a "$LOG_FILE"

# Step 4: Exit 0 (success) even with partial failures
exit 0
