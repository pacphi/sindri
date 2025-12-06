#!/usr/bin/env bash
# Phase 3: Validate all extensions (batched or sequential)
# Usage: validate-extensions.sh <provider> <profile> <target-id> <extensions-list>
# Returns: Sets validation results in GitHub Actions output

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/test-helpers.sh"

PROVIDER="${1:?Provider required}"
PROFILE="${2:-minimal}"
TARGET_ID="${3:?Target ID required}"
EXTENSIONS="${4:-}"

print_phase "3" "EXTENSION VALIDATION"

if [[ -z "$EXTENSIONS" ]]; then
    echo "No extensions to validate"
    echo "validation-results={}" >> "${GITHUB_OUTPUT:-/dev/null}"
    exit 0
fi

EXT_COUNT=$(echo "$EXTENSIONS" | wc -w | tr -d ' ')
RESULTS_JSON='{}'
VALIDATED=0
FAILED=0

# Use batched validation for remote providers (90-95% faster)
if [[ "${SINDRI_ENABLE_BATCHED_REMOTE_CALLS:-true}" == "true" ]] && [[ "$PROVIDER" != "docker" ]]; then
    echo "Using batched validation (single remote call, parallel execution)"
    echo "Extensions: $EXT_COUNT | Parallel jobs: ${SINDRI_PARALLEL_JOBS:-3}"
    echo ""

    # Execute batched validation script (baked into Docker image)
    set +e
    BATCH_OUTPUT=$("$SCRIPT_DIR/../providers/run-on-provider.sh" "$PROVIDER" "$TARGET_ID" \
        "/docker/scripts/batch-validate-extensions.sh $PROFILE" 2>&1)
    # shellcheck disable=SC2034  # BATCH_EXIT reserved for future error handling
    BATCH_EXIT=$?
    set -e

    # Parse results
    if echo "$BATCH_OUTPUT" | grep -q "NO_EXTENSIONS"; then
        echo "No extensions found in profile"
    else
        while IFS= read -r line; do
            if [[ "$line" =~ ^RESULT:(.+):(PASSED|FAILED)$ ]]; then
                ext="${BASH_REMATCH[1]}"
                status="${BASH_REMATCH[2]}"

                if [[ "$status" == "PASSED" ]]; then
                    echo "[$((VALIDATED + FAILED + 1))/$EXT_COUNT] ✅ $ext - PASSED"
                    RESULTS_JSON=$(echo "$RESULTS_JSON" | jq -c --arg ext "$ext" '.[$ext] = "passed"')
                    VALIDATED=$((VALIDATED + 1))
                else
                    echo "[$((VALIDATED + FAILED + 1))/$EXT_COUNT] ❌ $ext - FAILED"
                    RESULTS_JSON=$(echo "$RESULTS_JSON" | jq -c --arg ext "$ext" '.[$ext] = "failed"')
                    FAILED=$((FAILED + 1))
                fi
            fi
        done <<< "$BATCH_OUTPUT"

        echo ""
        echo "----------------------------------------"
        echo "Validation Summary: $VALIDATED passed, $FAILED failed"
        echo "Batched mode: Reduced $EXT_COUNT calls to 1 call (${SINDRI_PARALLEL_JOBS:-3} parallel validations)"
        echo "----------------------------------------"
    fi

else
    # Sequential validation (fallback for docker or when batching disabled)
    echo "Using sequential validation (one remote call per extension)"
    echo ""

    for ext in $EXTENSIONS; do
        echo "[$((VALIDATED + FAILED + 1))/$EXT_COUNT] Validating: $ext"
        set +e
        OUTPUT=$("$SCRIPT_DIR/../providers/run-on-provider.sh" "$PROVIDER" "$TARGET_ID" \
            "extension-manager validate $ext" 2>&1)
        EXIT_CODE=$?
        set -e

        if [[ $EXIT_CODE -eq 0 ]]; then
            echo "        ✅ $ext - PASSED"
            RESULTS_JSON=$(echo "$RESULTS_JSON" | jq -c --arg ext "$ext" '.[$ext] = "passed"')
            VALIDATED=$((VALIDATED + 1))
        else
            echo "        ❌ $ext - FAILED"
            echo "        Output: $OUTPUT"
            RESULTS_JSON=$(echo "$RESULTS_JSON" | jq -c --arg ext "$ext" '.[$ext] = "failed"')
            FAILED=$((FAILED + 1))
        fi
    done

    echo ""
    echo "----------------------------------------"
    echo "Validation Summary: $VALIDATED passed, $FAILED failed"
    echo "----------------------------------------"
fi

# Set GitHub Actions output
echo "validation-results=$RESULTS_JSON" >> "${GITHUB_OUTPUT:-/dev/null}"
echo "validated-count=$VALIDATED" >> "${GITHUB_OUTPUT:-/dev/null}"
echo "failed-count=$FAILED" >> "${GITHUB_OUTPUT:-/dev/null}"

# Exit with failure if any extensions failed
[[ $FAILED -eq 0 ]] && exit 0 || exit 1
