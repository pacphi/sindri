#!/usr/bin/env bash
# sindri-test.sh - Unified test script for Sindri CI
# Runs INSIDE the deployed container to test CLI and extension-manager functionality
#
# Usage: sindri-test.sh [--level quick|extension|profile|all] [--profile minimal] [--fail-fast]
#
# Test Levels:
#   quick      - CLI validation only (sindri, extension-manager commands)
#   extension  - Single extension lifecycle (install/validate/remove)
#   profile    - Profile lifecycle (install-profile/validate-all/remove)
#   all        - Run all levels sequentially
#
# Outputs structured results for CI parsing

set -euo pipefail

# Source common functions if available
if [[ -f /docker/lib/common.sh ]]; then
    source /docker/lib/common.sh
fi

# === Configuration ===
LEVEL="${LEVEL:-profile}"
PROFILE="${PROFILE:-minimal}"
FAIL_FAST="${FAIL_FAST:-true}"
TEST_EXTENSION="${TEST_EXTENSION:-nodejs}"  # Default extension for single extension tests

# Counters
PASSED=0
FAILED=0

# === Helper Functions ===
print_header() {
    echo ""
    echo "========================================"
    echo "  $1"
    echo "========================================"
    echo ""
}

run_test() {
    local name="$1"
    # shellcheck disable=SC2178  # cmd is a string, not an array
    local cmd="$2"
    local start
    start=$(date +%s)

    set +e
    local output
    # shellcheck disable=SC2128  # cmd is a string, not an array
    output=$(eval "$cmd" 2>&1)
    local exit_code=$?
    set -e

    # Handle SIGPIPE (141) - happens when grep -q exits early after finding match
    # This is actually a success case (see: https://stackoverflow.com/q/19120263)
    if [[ $exit_code -eq 141 ]]; then
        exit_code=0
    fi

    local duration=$(($(date +%s) - start))

    if [[ $exit_code -eq 0 ]]; then
        PASSED=$((PASSED + 1))
        echo "PASS: $name (${duration}s)"
        return 0
    else
        FAILED=$((FAILED + 1))
        echo "FAIL: $name (${duration}s)"
        echo "  Error: $output"

        if [[ "$FAIL_FAST" == "true" ]]; then
            echo ""
            echo "RESULT:FAILED"
            echo "Summary: $PASSED passed, $FAILED failed"
            exit 1
        fi
        return 1
    fi
}

# === Quick Tests (CLI Validation) ===
run_quick_tests() {
    print_header "Quick Tests - CLI Validation"

    run_test "sindri-version" "sindri --version"
    run_test "sindri-help" "sindri --help"
    run_test "extension-manager-list" "extension-manager list"
    run_test "extension-manager-profiles" "extension-manager list-profiles"
    run_test "extension-manager-categories" "extension-manager list-categories"
    run_test "mise-available" "command -v mise"
    run_test "yq-available" "command -v yq"
}

# === Extension Lifecycle Tests ===
run_extension_lifecycle_tests() {
    print_header "Extension Lifecycle - Single Extension"

    # Step 1: List (verify registry accessible)
    run_test "list" "extension-manager list"

    # Step 2: PRE-CHECK - Verify extension NOT already installed
    echo "# Pre-check: Verifying $TEST_EXTENSION is NOT installed"
    run_test "pre-check-$TEST_EXTENSION" \
        "extension-manager status $TEST_EXTENSION | grep -q 'Status: Not installed' || (echo 'DIRTY STATE DETECTED - $TEST_EXTENSION is already installed!' && exit 1)"

    # Step 3: Install single extension
    run_test "install-$TEST_EXTENSION" "extension-manager install $TEST_EXTENSION"

    # Step 4: Validate extension
    run_test "validate-$TEST_EXTENSION" "extension-manager validate $TEST_EXTENSION"

    # Step 5: Check status
    run_test "status-$TEST_EXTENSION" \
        "extension-manager status $TEST_EXTENSION | grep -q 'Status: Installed'"

    # Step 6: Verify tool works
    local verify_cmd
    case "$TEST_EXTENSION" in
        nodejs) verify_cmd="node --version" ;;
        python) verify_cmd="python --version" ;;
        golang) verify_cmd="go version" ;;
        ruby) verify_cmd="ruby --version" ;;
        rust) verify_cmd="rustc --version" ;;
        *) verify_cmd="echo 'No verification command for $TEST_EXTENSION'" ;;
    esac
    run_test "verify-$TEST_EXTENSION" "$verify_cmd"

    # Step 7: Idempotency - Reinstall extension
    echo "# Idempotency: Testing reinstall"
    run_test "idempotency-reinstall-$TEST_EXTENSION" "extension-manager install $TEST_EXTENSION"

    # Step 8: Idempotency - Revalidate
    run_test "idempotency-revalidate-$TEST_EXTENSION" "extension-manager validate $TEST_EXTENSION"

    # Step 9: Generate BOM
    run_test "bom" "extension-manager bom"

    # Step 10: Remove extension
    run_test "remove-$TEST_EXTENSION" "extension-manager remove $TEST_EXTENSION"

    # Step 11: Verify removed
    run_test "verify-removed" \
        "extension-manager status $TEST_EXTENSION | grep -q 'Status: Not installed'"
}

# === Profile Lifecycle Tests ===
run_profile_lifecycle_tests() {
    print_header "Profile Lifecycle - $PROFILE Profile"

    # Step 1: List (verify registry accessible)
    run_test "list" "extension-manager list"

    # Step 2: PRE-CHECK - Verify NO profile extensions are installed
    echo "# Pre-check: Verifying NO extensions from '$PROFILE' profile are installed"
    local extensions
    extensions=$(yq ".profiles.${PROFILE}.extensions[]" /docker/lib/profiles.yaml 2>/dev/null || echo "")

    if [[ -z "$extensions" ]]; then
        echo "FAIL: pre-check-profile (0s)"
        echo "  Error: Profile '$PROFILE' not found or has no extensions"
        FAILED=$((FAILED + 1))
        [[ "$FAIL_FAST" == "true" ]] && exit 1
        return 1
    fi

    # Check each extension is NOT installed
    local all_clean=true
    for ext in $extensions; do
        if extension-manager status "$ext" 2>/dev/null | grep -q 'Status: Installed'; then
            echo "  ERROR: $ext is already installed!"
            all_clean=false
        fi
    done

    if [[ "$all_clean" == "true" ]]; then
        PASSED=$((PASSED + 1))
        echo "PASS: pre-check-profile (1s)      # Verified NO extensions installed"
    else
        FAILED=$((FAILED + 1))
        echo "FAIL: pre-check-profile (1s)"
        echo "  Error: DIRTY STATE DETECTED - Some profile extensions are already installed!"
        echo "  This indicates stale volumes, autoInstall misconfiguration, or incomplete cleanup."
        [[ "$FAIL_FAST" == "true" ]] && exit 1
        return 1
    fi

    # Step 3: Install profile
    run_test "install-profile-$PROFILE" "extension-manager install-profile $PROFILE"

    # Step 4: Validate all extensions
    run_test "validate-all" "extension-manager validate-all"

    # Step 5: Check status of all extensions
    run_test "status-all" "extension-manager status"

    # Step 6: Verify tools work
    echo "# Verifying profile tools are functional"
    local verify_passed=true
    for ext in $extensions; do
        local verify_cmd
        case "$ext" in
            nodejs) verify_cmd="node --version && npm --version" ;;
            python) verify_cmd="python --version && pip --version" ;;
            golang) verify_cmd="go version" ;;
            ruby) verify_cmd="ruby --version && gem --version" ;;
            rust) verify_cmd="rustc --version && cargo --version" ;;
            *) continue ;;  # Skip extensions without verification commands
        esac

        set +e
        eval "$verify_cmd" >/dev/null 2>&1
        if [[ $? -ne 0 ]]; then
            verify_passed=false
            echo "  WARNING: $ext verification failed"
        fi
        set -e
    done

    if [[ "$verify_passed" == "true" ]]; then
        PASSED=$((PASSED + 1))
        echo "PASS: verify-tools (2s)"
    else
        FAILED=$((FAILED + 1))
        echo "FAIL: verify-tools (2s)"
        [[ "$FAIL_FAST" == "true" ]] && exit 1
    fi

    # Step 7: Idempotency - Reinstall profile
    echo "# Idempotency: Testing reinstall of profile"
    run_test "idempotency-reinstall-profile" "extension-manager install-profile $PROFILE"

    # Step 8: Idempotency - Revalidate all
    run_test "idempotency-revalidate-all" "extension-manager validate-all"

    # Step 9: Generate BOM
    run_test "bom" "extension-manager bom"

    # Step 10: Remove all profile extensions
    echo "# Removing all profile extensions"
    local remove_passed=true
    for ext in $extensions; do
        set +e
        extension-manager remove "$ext" >/dev/null 2>&1
        if [[ $? -ne 0 ]]; then
            remove_passed=false
            echo "  WARNING: Failed to remove $ext"
        fi
        set -e
    done

    if [[ "$remove_passed" == "true" ]]; then
        PASSED=$((PASSED + 1))
        echo "PASS: remove-all (4s)"
    else
        FAILED=$((FAILED + 1))
        echo "FAIL: remove-all (4s)"
        [[ "$FAIL_FAST" == "true" ]] && exit 1
    fi

    # Step 11: Verify all removed
    local all_removed=true
    for ext in $extensions; do
        if extension-manager status "$ext" 2>/dev/null | grep -q 'Status: Installed'; then
            all_removed=false
            echo "  ERROR: $ext is still installed!"
        fi
    done

    if [[ "$all_removed" == "true" ]]; then
        PASSED=$((PASSED + 1))
        echo "PASS: verify-removed (1s)"
    else
        FAILED=$((FAILED + 1))
        echo "FAIL: verify-removed (1s)"
        [[ "$FAIL_FAST" == "true" ]] && exit 1
    fi
}

# === Main ===
main() {
    echo "Sindri Test Suite"
    echo "Level: $LEVEL | Profile: $PROFILE | Fail-Fast: $FAIL_FAST"
    echo "========================================"

    case "$LEVEL" in
        quick)
            run_quick_tests
            ;;
        extension)
            run_extension_lifecycle_tests
            ;;
        profile)
            run_profile_lifecycle_tests
            ;;
        all)
            run_quick_tests
            run_extension_lifecycle_tests
            run_profile_lifecycle_tests
            ;;
        *)
            echo "ERROR: Unknown test level: $LEVEL"
            echo "Valid levels: quick, extension, profile, all"
            exit 1
            ;;
    esac

    echo ""
    echo "========================================"
    echo "  RESULTS SUMMARY"
    echo "========================================"
    echo "Passed: $PASSED"
    echo "Failed: $FAILED"
    echo ""

    if [[ $FAILED -gt 0 ]]; then
        echo "RESULT:FAILED"
        exit 1
    else
        echo "RESULT:PASSED"
        exit 0
    fi
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --level)
            LEVEL="$2"
            shift 2
            ;;
        --profile)
            PROFILE="$2"
            shift 2
            ;;
        --extension)
            TEST_EXTENSION="$2"
            shift 2
            ;;
        --fail-fast)
            FAIL_FAST="true"
            shift
            ;;
        --no-fail-fast)
            FAIL_FAST="false"
            shift
            ;;
        *)
            echo "Unknown argument: $1"
            echo "Usage: sindri-test.sh [--level quick|extension|profile|all] [--profile minimal] [--extension nodejs] [--fail-fast|--no-fail-fast]"
            exit 1
            ;;
    esac
done

main
