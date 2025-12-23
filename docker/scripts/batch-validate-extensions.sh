#!/usr/bin/env bash
# Batched extension validation script for CI testing
# Reduces N remote calls to 1 call (90-95% SSH/exec overhead reduction)
# Usage: batch-validate-extensions.sh <profile-name>
# Runs remotely on provider infrastructure with parallel validation

set -euo pipefail

# Get parallel jobs setting (provider-specific, set by caller)
PARALLEL_JOBS="${SINDRI_PARALLEL_JOBS:-3}"

# Helper to log structured results
log_result() {
    echo "RESULT:${1}:${2}"  # extension:status
}

# Validation function (runs in parallel via xargs)
validate_one() {
    local ext="$1"
    local timeout="${SINDRI_VALIDATION_TIMEOUT:-30}"

    if timeout "$timeout" extension-manager validate "$ext" >/dev/null 2>&1; then
        log_result "$ext" "PASSED"
        return 0
    else
        log_result "$ext" "FAILED"
        return 1
    fi
}

export -f validate_one
export -f log_result
export SINDRI_VALIDATION_TIMEOUT

# Get extensions from profile
PROFILE="${1:-minimal}"
EXTENSIONS=$(yq ".profiles.${PROFILE}.extensions[]" /docker/lib/profiles.yaml 2>/dev/null || echo "")

if [[ -z "$EXTENSIONS" ]]; then
    echo "NO_EXTENSIONS"
    exit 0
fi

# Run validations in parallel
echo "$EXTENSIONS" | xargs -P "$PARALLEL_JOBS" -I {} bash -c 'validate_one "$@"' _ {}

echo "VALIDATION_COMPLETE"
