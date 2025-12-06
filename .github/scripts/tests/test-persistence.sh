#!/usr/bin/env bash
# Test file persistence across restarts
# Usage: test-persistence.sh <provider> <app-name-or-target-id>
# Tests that files survive machine/container restarts

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/test-helpers.sh"

PROVIDER="${1:?Provider required}"
TARGET_ID="${2:?Target ID (app name or container) required}"

log_info "Testing file persistence across restarts..."

# Create unique test file
TEST_FILE="\${WORKSPACE:-\$HOME/workspace}/test-persistence-$(date +%s).txt"
TEST_CONTENT="persistence-test-$(date +%s)"

# Create test file on remote
case "$PROVIDER" in
    fly)
        flyctl ssh console -a "$TARGET_ID" --command "echo '$TEST_CONTENT' > $TEST_FILE"

        # Get machine ID
        MACHINE_ID=$(flyctl machine list -a "$TARGET_ID" --json | jq -r '.[0].id')

        # Restart machine
        log_info "Restarting Fly.io machine..."
        flyctl machine restart "$MACHINE_ID" -a "$TARGET_ID"

        # Wait for VM to come back
        wait_for_vm "$TARGET_ID"

        # Check if file persists
        ACTUAL_CONTENT=$(flyctl ssh console -a "$TARGET_ID" --command "cat $TEST_FILE")
        ;;

    docker)
        docker exec "$TARGET_ID" bash -c "echo '$TEST_CONTENT' > $TEST_FILE"

        # Restart container
        log_info "Restarting Docker container..."
        docker restart "$TARGET_ID"
        sleep 5

        # Check if file persists
        ACTUAL_CONTENT=$(docker exec "$TARGET_ID" cat "$TEST_FILE")
        ;;

    *)
        log_warning "Persistence test not implemented for provider: $PROVIDER"
        exit 0
        ;;
esac

# Validate persistence
if [[ "$ACTUAL_CONTENT" == "$TEST_CONTENT" ]]; then
    log_success "Persistence test PASSED - file survived restart"

    # Cleanup
    case "$PROVIDER" in
        fly) flyctl ssh console -a "$TARGET_ID" --command "rm -f $TEST_FILE" ;;
        docker) docker exec "$TARGET_ID" rm -f "$TEST_FILE" ;;
    esac

    exit 0
else
    log_error "Persistence test FAILED - file content mismatch or file lost"
    log_error "Expected: $TEST_CONTENT"
    log_error "Actual: $ACTUAL_CONTENT"
    exit 1
fi
