#!/usr/bin/env bash
# Test individual extension idempotency (different from profile-level test-idempotency.sh)
# Usage: test-extension-idempotency.sh <provider> <target-id> <extension-name>
# Tests that installing an extension twice doesn't break it

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/test-helpers.sh"

PROVIDER="${1:?Provider required}"
TARGET_ID="${2:?Target ID required}"
EXTENSION="${3:?Extension name required}"

log_info "Testing idempotency for extension: $EXTENSION"

# First installation
log_info "First installation..."
set +e
"$SCRIPT_DIR/../providers/run-on-provider.sh" "$PROVIDER" "$TARGET_ID" \
    "extension-manager install $EXTENSION"
FIRST_STATUS=$?
set -e

if [[ $FIRST_STATUS -ne 0 ]]; then
    log_error "First installation failed"
    exit 1
fi

# Second installation (should be idempotent - no errors)
log_info "Second installation (idempotency check)..."
set +e
"$SCRIPT_DIR/../providers/run-on-provider.sh" "$PROVIDER" "$TARGET_ID" \
    "extension-manager install $EXTENSION"
SECOND_STATUS=$?
set -e

if [[ $SECOND_STATUS -ne 0 ]]; then
    log_error "Second installation failed - not idempotent!"
    exit 1
fi

# Validate extension still works
log_info "Validating extension after reinstall..."
if "$SCRIPT_DIR/../providers/run-on-provider.sh" "$PROVIDER" "$TARGET_ID" \
    "extension-manager validate $EXTENSION" &>/dev/null; then
    log_success "Idempotency test PASSED for $EXTENSION"
    exit 0
else
    log_error "Validation failed after reinstall"
    exit 1
fi
