#!/usr/bin/env bash
# Batched functionality tests for extensions
# Reduces N remote calls to 1 call for Phase 4 functionality testing
# Usage: batch-functionality-tests.sh <profile-name> <extensions-list>
# Runs remotely on provider infrastructure with parallel testing

set -euo pipefail

# shellcheck disable=SC2034  # PROFILE used by caller context
PROFILE="${1:-minimal}"
EXTENSIONS="${2:?Extensions list required}"
# shellcheck disable=SC2034  # PARALLEL_JOBS may be used in future parallel implementation
PARALLEL_JOBS="${SINDRI_PARALLEL_JOBS:-3}"

# Helper to log structured results
log_result() {
    echo "RESULT:${1}:${2}:${3}"  # tool:status:version
}

# Test tool functionality
test_tool() {
    local ext="$1"
    local timeout="${SINDRI_VALIDATION_TIMEOUT:-5}"

    case "$ext" in
        nodejs)
            if NODE_VER=$(timeout "$timeout" node --version 2>&1); then
                log_result "node" "PASSED" "$NODE_VER"
            else
                log_result "node" "FAILED" "timeout_or_error"
            fi

            if NPM_VER=$(timeout "$timeout" npm --version 2>&1); then
                log_result "npm" "PASSED" "$NPM_VER"
            else
                log_result "npm" "FAILED" "timeout_or_error"
            fi
            ;;
        python)
            if PY_VER=$(timeout "$timeout" python --version 2>&1); then
                log_result "python" "PASSED" "$PY_VER"
            else
                log_result "python" "FAILED" "timeout_or_error"
            fi

            if PIP_VER=$(timeout "$timeout" pip --version 2>&1); then
                log_result "pip" "PASSED" "$PIP_VER"
            else
                log_result "pip" "FAILED" "timeout_or_error"
            fi
            ;;
        golang)
            if GO_VER=$(timeout "$timeout" go version 2>&1); then
                log_result "go" "PASSED" "$GO_VER"
            else
                log_result "go" "FAILED" "timeout_or_error"
            fi
            ;;
        rust)
            if RUSTC_VER=$(timeout "$timeout" rustc --version 2>&1); then
                log_result "rustc" "PASSED" "$RUSTC_VER"
            else
                log_result "rustc" "FAILED" "timeout_or_error"
            fi

            if CARGO_VER=$(timeout "$timeout" cargo --version 2>&1); then
                log_result "cargo" "PASSED" "$CARGO_VER"
            else
                log_result "cargo" "FAILED" "timeout_or_error"
            fi
            ;;
        ruby)
            if RUBY_VER=$(timeout "$timeout" ruby --version 2>&1); then
                log_result "ruby" "PASSED" "$RUBY_VER"
            else
                log_result "ruby" "FAILED" "timeout_or_error"
            fi

            if GEM_VER=$(timeout "$timeout" gem --version 2>&1); then
                log_result "gem" "PASSED" "$GEM_VER"
            else
                log_result "gem" "FAILED" "timeout_or_error"
            fi
            ;;
        *)
            # No specific functionality tests for this extension
            ;;
    esac
}

export -f test_tool
export -f log_result
export SINDRI_VALIDATION_TIMEOUT

# Run functionality tests for all extensions
for ext in $EXTENSIONS; do
    test_tool "$ext"
done

echo "FUNCTIONALITY_TESTS_COMPLETE"
