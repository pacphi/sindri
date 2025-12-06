#!/bin/bash
# Test helper functions for CI/CD

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Retry function with exponential backoff
retry_with_backoff() {
    local max_attempts="${1:-3}"
    local delay="${2:-1}"
    local max_delay="${3:-30}"
    local command="${*:4}"
    local attempt=0

    while [[ $attempt -lt $max_attempts ]]; do
        attempt=$((attempt + 1))

        if eval "$command"; then
            return 0
        fi

        if [[ $attempt -lt $max_attempts ]]; then
            log_warning "Command failed (attempt $attempt/$max_attempts). Retrying in ${delay}s..."
            sleep "$delay"
            delay=$((delay * 2))
            if [[ $delay -gt $max_delay ]]; then
                delay=$max_delay
            fi
        fi
    done

    log_error "Command failed after $max_attempts attempts"
    return 1
}

# Check if command exists on remote VM
check_command_exists() {
    local app_name="$1"
    local command="$2"

    if flyctl ssh console -a "$app_name" --command "command -v $command" &>/dev/null; then
        log_success "$command is available"
        return 0
    else
        log_error "$command is not available"
        return 1
    fi
}

# Run command on VM and capture output
run_on_vm() {
    local app_name="$1"
    local command="$2"

    log_info "Running on VM: $command"
    flyctl ssh console -a "$app_name" --command "$command"
}

# Check if extension is installed
is_extension_installed() {
    local app_name="$1"
    local extension="$2"

    run_on_vm "$app_name" "extension-manager status $extension" | grep -q "installed"
}

# Wait for VM to be ready
wait_for_vm() {
    local app_name="$1"
    local max_wait="${2:-300}"  # 5 minutes default
    local elapsed=0

    log_info "Waiting for VM $app_name to be ready..."

    while [[ $elapsed -lt $max_wait ]]; do
        if flyctl ssh console -a "$app_name" --command "echo 'ready'" &>/dev/null; then
            log_success "VM $app_name is ready"
            return 0
        fi

        sleep 5
        elapsed=$((elapsed + 5))
        echo -n "."
    done

    log_error "VM $app_name did not become ready within ${max_wait}s"
    return 1
}

# Note: test_persistence and test_idempotency have been moved to standalone scripts:
# - .github/scripts/tests/test-persistence.sh
# - .github/scripts/tests/test-extension-idempotency.sh
# These are now called directly rather than as functions

# Compare versions
version_gt() {
    test "$(printf '%s\n' "$@" | sort -V | head -n 1)" != "$1"
}

# =============================================================================
# Additional Test Helpers (for modular phase scripts)
# =============================================================================

# Print phase header
print_phase() {
    local phase_num="$1"
    local phase_name="$2"
    echo ""
    echo "========================================"
    echo "  PHASE ${phase_num}: ${phase_name}"
    echo "========================================"
    echo ""
}

# Log structured result (for output parsing)
log_result() {
    local component="$1"
    local status="$2"
    local message="${3:-}"

    if [[ -n "$message" ]]; then
        echo "RESULT:${component}:${status}:${message}"
    else
        echo "RESULT:${component}:${status}"
    fi
}

# Export functions for use in other scripts
export -f log_info log_success log_warning log_error
export -f retry_with_backoff check_command_exists run_on_vm
export -f is_extension_installed wait_for_vm version_gt
export -f print_phase log_result