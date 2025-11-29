#!/bin/bash
# Test all extensions locally or in CI

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/test-helpers.sh"

# Configuration
TEST_MODE="${1:-local}"  # local or ci
APP_NAME="${2:-sindri-test-all}"

# Get list of all extensions
EXTENSIONS_DIR="$SCRIPT_DIR/../../docker/lib/extensions"
EXTENSIONS=$(find "$EXTENSIONS_DIR" -mindepth 1 -maxdepth 1 -type d -exec basename {} \; | sort)

# Track results
declare -A RESULTS

log_info "Testing all extensions in $TEST_MODE mode"
log_info "Found extensions: $(echo "$EXTENSIONS" | tr '\n' ' ')"

# Test each extension
for ext in $EXTENSIONS; do
    log_info "Testing extension: $ext"

    if [[ "$TEST_MODE" == "ci" ]]; then
        # In CI, use the complete test script
        if bash "$SCRIPT_DIR/extensions/test-extension-complete.sh" "$ext" "$APP_NAME"; then
            RESULTS[$ext]="PASSED"
            log_success "$ext: PASSED"
        else
            RESULTS[$ext]="FAILED"
            log_error "$ext: FAILED"
        fi
    else
        # In local mode, just validate the extension structure
        ext_dir="$EXTENSIONS_DIR/$ext"

        # Check for required files
        if [[ -f "$ext_dir/extension.yaml" ]]; then
            if [[ -f "$ext_dir/install.sh" ]]; then
                RESULTS[$ext]="VALID"
                log_success "$ext: VALID"
            else
                RESULTS[$ext]="MISSING_INSTALL"
                log_error "$ext: Missing install.sh"
            fi
        else
            RESULTS[$ext]="MISSING_YAML"
            log_error "$ext: Missing extension.yaml"
        fi
    fi
done

# Print summary
echo ""
echo "========================================"
echo "           TEST SUMMARY"
echo "========================================"

PASSED=0
FAILED=0

for ext in $EXTENSIONS; do
    status="${RESULTS[$ext]}"
    case "$status" in
        PASSED|VALID)
            echo "✓ $ext: $status"
            PASSED=$((PASSED + 1))
            ;;
        *)
            echo "✗ $ext: $status"
            FAILED=$((FAILED + 1))
            ;;
    esac
done

echo ""
echo "Total: $((PASSED + FAILED)) extensions"
echo "Passed: $PASSED"
echo "Failed: $FAILED"

if [[ $FAILED -eq 0 ]]; then
    log_success "All extensions passed!"
    exit 0
else
    log_error "Some extensions failed"
    exit 1
fi