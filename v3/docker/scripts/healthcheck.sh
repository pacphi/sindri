#!/bin/bash
# ==============================================================================
# Sindri v3 Health Check Script
# ==============================================================================
# Verifies container health status including:
# 1. SSH daemon is running
# 2. Extension installation status
# 3. Critical directories exist
# 4. Sindri CLI is functional
#
# Exit codes:
#   0 - Healthy
#   1 - Unhealthy
# ==============================================================================

set -euo pipefail

# Configuration
ALT_HOME="${ALT_HOME:-/alt/home/developer}"
SINDRI_HOME="${ALT_HOME}/.sindri"
SSH_PORT="${SSH_PORT:-2222}"
BOOTSTRAP_MARKER="${SINDRI_HOME}/bootstrap-complete"
INSTALL_LOG="${SINDRI_HOME}/logs/install.log"

# Counters for reporting
CHECKS_PASSED=0
CHECKS_FAILED=0

# ==============================================================================
# Helper Functions
# ==============================================================================

check_pass() {
    CHECKS_PASSED=$((CHECKS_PASSED + 1))
    echo "✓ $1"
}

check_fail() {
    CHECKS_FAILED=$((CHECKS_FAILED + 1))
    echo "✗ $1" >&2
}

# ==============================================================================
# Health Checks
# ==============================================================================

# Check 1: SSH daemon is running and listening
if ss -tln 2>/dev/null | grep -q ":${SSH_PORT}"; then
    check_pass "SSH daemon is listening on port ${SSH_PORT}"
else
    check_fail "SSH daemon is not listening on port ${SSH_PORT}"
fi

# Check 2: Sindri CLI is available and functional
if command -v sindri &> /dev/null; then
    if sindri --version &> /dev/null; then
        check_pass "Sindri CLI is functional"
    else
        check_fail "Sindri CLI exists but fails to execute"
    fi
else
    check_fail "Sindri CLI not found in PATH"
fi

# Check 3: Critical directories exist
if [[ -d "$ALT_HOME" ]]; then
    check_pass "Home directory exists: ${ALT_HOME}"
else
    check_fail "Home directory missing: ${ALT_HOME}"
fi

if [[ -d "$SINDRI_HOME" ]]; then
    check_pass "Sindri directory exists: ${SINDRI_HOME}"
else
    check_fail "Sindri directory missing: ${SINDRI_HOME}"
fi

# Check 4: Extension installation status
if [[ "${SKIP_AUTO_INSTALL:-false}" == "true" ]]; then
    check_pass "Extension auto-install disabled (expected)"
elif [[ -f "$BOOTSTRAP_MARKER" ]]; then
    check_pass "Extension installation complete"
else
    # Check if installation is in progress
    if [[ -f "$INSTALL_LOG" ]]; then
        # Check if install process is still running
        if pgrep -f "sindri.*install" > /dev/null 2>&1; then
            check_pass "Extension installation in progress"
        else
            # Installation started but not marked complete and process not running
            # Check the log for errors
            if tail -20 "$INSTALL_LOG" 2>/dev/null | grep -qi "error\|failed"; then
                check_fail "Extension installation failed (see ~/.sindri/logs/install.log)"
            else
                check_pass "Extension installation completed (marker pending)"
            fi
        fi
    else
        # No log file yet - installation hasn't started or just started
        check_pass "Extension installation not yet started"
    fi
fi

# Check 5: Home directory is writable
if touch "${ALT_HOME}/.health_check_test" 2>/dev/null; then
    rm -f "${ALT_HOME}/.health_check_test"
    check_pass "Home directory is writable"
else
    check_fail "Home directory is not writable"
fi

# Check 6: Developer user exists
if id -u developer &> /dev/null; then
    check_pass "Developer user exists"
else
    check_fail "Developer user does not exist"
fi

# ==============================================================================
# Summary and Exit
# ==============================================================================

TOTAL_CHECKS=$((CHECKS_PASSED + CHECKS_FAILED))

echo ""
echo "Health check summary: ${CHECKS_PASSED}/${TOTAL_CHECKS} checks passed"

if [[ $CHECKS_FAILED -eq 0 ]]; then
    echo "Status: HEALTHY"
    exit 0
else
    echo "Status: UNHEALTHY (${CHECKS_FAILED} checks failed)"
    exit 1
fi
