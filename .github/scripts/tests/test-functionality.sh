#!/usr/bin/env bash
# Phase 4: Test tool functionality (node, npm, python, pip, etc.)
# Usage: test-functionality.sh <provider> <profile> <target-id> <extensions-list>
# Returns: Sets functionality test results in GitHub Actions output

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/test-helpers.sh"

PROVIDER="${1:?Provider required}"
# shellcheck disable=SC2034  # PROFILE used by caller for context
PROFILE="${2:-minimal}"
TARGET_ID="${3:?Target ID required}"
EXTENSIONS="${4:-}"

print_phase "4" "FUNCTIONALITY TESTS"

if [[ -z "$EXTENSIONS" ]]; then
    echo "No extensions to test"
    echo "functionality-results={}" >> "${GITHUB_OUTPUT:-/dev/null}"
    exit 0
fi

RESULTS_JSON='{}'
FUNC_TESTED=0
FUNC_FAILED=0

# Test extension-specific tool commands
for ext in $EXTENSIONS; do
    echo "[$ext] Testing functionality..."

    case "$ext" in
        nodejs)
            set +e
            "$SCRIPT_DIR/../providers/run-on-provider.sh" "$PROVIDER" "$TARGET_ID" "node --version" &>/dev/null && \
                echo "  ✅ node command" || { echo "  ❌ node command"; FUNC_FAILED=$((FUNC_FAILED + 1)); }
            "$SCRIPT_DIR/../providers/run-on-provider.sh" "$PROVIDER" "$TARGET_ID" "npm --version" &>/dev/null && \
                echo "  ✅ npm command" || { echo "  ❌ npm command"; FUNC_FAILED=$((FUNC_FAILED + 1)); }
            set -e
            FUNC_TESTED=$((FUNC_TESTED + 2))
            RESULTS_JSON=$(echo "$RESULTS_JSON" | jq -c '.nodejs_tested = true')
            ;;
        python)
            set +e
            "$SCRIPT_DIR/../providers/run-on-provider.sh" "$PROVIDER" "$TARGET_ID" "python --version" &>/dev/null && \
                echo "  ✅ python command" || { echo "  ❌ python command"; FUNC_FAILED=$((FUNC_FAILED + 1)); }
            "$SCRIPT_DIR/../providers/run-on-provider.sh" "$PROVIDER" "$TARGET_ID" "pip --version" &>/dev/null && \
                echo "  ✅ pip command" || { echo "  ❌ pip command"; FUNC_FAILED=$((FUNC_FAILED + 1)); }
            set -e
            FUNC_TESTED=$((FUNC_TESTED + 2))
            RESULTS_JSON=$(echo "$RESULTS_JSON" | jq -c '.python_tested = true')
            ;;
        golang)
            set +e
            "$SCRIPT_DIR/../providers/run-on-provider.sh" "$PROVIDER" "$TARGET_ID" "go version" &>/dev/null && \
                echo "  ✅ go command" || { echo "  ❌ go command"; FUNC_FAILED=$((FUNC_FAILED + 1)); }
            set -e
            FUNC_TESTED=$((FUNC_TESTED + 1))
            RESULTS_JSON=$(echo "$RESULTS_JSON" | jq -c '.golang_tested = true')
            ;;
        rust)
            set +e
            "$SCRIPT_DIR/../providers/run-on-provider.sh" "$PROVIDER" "$TARGET_ID" "rustc --version" &>/dev/null && \
                echo "  ✅ rustc command" || { echo "  ❌ rustc command"; FUNC_FAILED=$((FUNC_FAILED + 1)); }
            "$SCRIPT_DIR/../providers/run-on-provider.sh" "$PROVIDER" "$TARGET_ID" "cargo --version" &>/dev/null && \
                echo "  ✅ cargo command" || { echo "  ❌ cargo command"; FUNC_FAILED=$((FUNC_FAILED + 1)); }
            set -e
            FUNC_TESTED=$((FUNC_TESTED + 2))
            RESULTS_JSON=$(echo "$RESULTS_JSON" | jq -c '.rust_tested = true')
            ;;
        ruby)
            set +e
            "$SCRIPT_DIR/../providers/run-on-provider.sh" "$PROVIDER" "$TARGET_ID" "ruby --version" &>/dev/null && \
                echo "  ✅ ruby command" || { echo "  ❌ ruby command"; FUNC_FAILED=$((FUNC_FAILED + 1)); }
            "$SCRIPT_DIR/../providers/run-on-provider.sh" "$PROVIDER" "$TARGET_ID" "gem --version" &>/dev/null && \
                echo "  ✅ gem command" || { echo "  ❌ gem command"; FUNC_FAILED=$((FUNC_FAILED + 1)); }
            set -e
            FUNC_TESTED=$((FUNC_TESTED + 2))
            RESULTS_JSON=$(echo "$RESULTS_JSON" | jq -c '.ruby_tested = true')
            ;;
        *)
            echo "  (No specific functionality tests defined for $ext)"
            ;;
    esac
done

echo ""
echo "----------------------------------------"
echo "Functionality Tests: $FUNC_TESTED tested, $FUNC_FAILED failed"
echo "----------------------------------------"

# Set GitHub Actions output
echo "functionality-results=$RESULTS_JSON" >> "${GITHUB_OUTPUT:-/dev/null}"
echo "func-tested=$FUNC_TESTED" >> "${GITHUB_OUTPUT:-/dev/null}"
echo "func-failed=$FUNC_FAILED" >> "${GITHUB_OUTPUT:-/dev/null}"

# Exit with failure if any tests failed
[[ $FUNC_FAILED -eq 0 ]] && exit 0 || exit 1
