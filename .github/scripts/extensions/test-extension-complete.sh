#!/bin/bash
# Complete test suite for extensions

set -euo pipefail

# Source helpers
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/test-helpers.sh"
source "$SCRIPT_DIR/../lib/assertions.sh"

# Arguments
EXTENSION="${1:-}"
APP_NAME="${2:-}"

if [[ -z "$EXTENSION" || -z "$APP_NAME" ]]; then
    log_error "Usage: $0 <extension> <app-name>"
    exit 1
fi

log_info "Starting complete test suite for extension: $EXTENSION"

# Test suite results
TESTS_PASSED=0
TESTS_FAILED=0

# Function to run a test
run_test() {
    local test_name="$1"
    local test_command="$2"

    echo -n "  Testing $test_name... "
    if eval "$test_command" &>/dev/null; then
        echo "✓"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo "✗"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
}

# 1. Installation Test
log_info "Phase 1: Installation Tests"
run_test "installation" "run_on_vm '$APP_NAME' 'extension-manager install $EXTENSION'"
run_test "status check" "is_extension_installed '$APP_NAME' '$EXTENSION'"

# 2. Validation Test
log_info "Phase 2: Validation Tests"
run_test "validation" "run_on_vm '$APP_NAME' 'extension-manager validate $EXTENSION'"

# 3. Extension-specific functionality tests
log_info "Phase 3: Functionality Tests"

case "$EXTENSION" in
    nodejs)
        run_test "node command" "assert_command node '$APP_NAME'"
        run_test "npm command" "assert_command npm '$APP_NAME'"
        run_test "node version" "run_on_vm '$APP_NAME' 'node --version'"
        ;;

    python)
        run_test "python command" "assert_command python '$APP_NAME'"
        run_test "pip command" "assert_command pip '$APP_NAME'"
        run_test "python version" "run_on_vm '$APP_NAME' 'python --version'"
        ;;

    docker)
        run_test "docker command" "assert_command docker '$APP_NAME'"
        run_test "docker compose" "run_on_vm '$APP_NAME' 'docker compose version'"
        run_test "docker version" "run_on_vm '$APP_NAME' 'docker --version'"
        ;;

    golang)
        run_test "go command" "assert_command go '$APP_NAME'"
        run_test "go version" "run_on_vm '$APP_NAME' 'go version'"
        ;;

    rust)
        run_test "rustc command" "assert_command rustc '$APP_NAME'"
        run_test "cargo command" "assert_command cargo '$APP_NAME'"
        run_test "rust version" "run_on_vm '$APP_NAME' 'rustc --version'"
        ;;

    ruby)
        run_test "ruby command" "assert_command ruby '$APP_NAME'"
        run_test "gem command" "assert_command gem '$APP_NAME'"
        run_test "bundle command" "assert_command bundle '$APP_NAME'"
        run_test "ruby version" "run_on_vm '$APP_NAME' 'ruby --version'"
        ;;

    php)
        run_test "php command" "assert_command php '$APP_NAME'"
        run_test "composer command" "assert_command composer '$APP_NAME'"
        run_test "php version" "run_on_vm '$APP_NAME' 'php --version'"
        ;;

    nodejs-devtools)
        run_test "typescript" "assert_command tsc '$APP_NAME'"
        run_test "eslint" "assert_command eslint '$APP_NAME'"
        run_test "prettier" "assert_command prettier '$APP_NAME'"
        ;;

    github-cli)
        run_test "gh command" "assert_command gh '$APP_NAME'"
        run_test "gh version" "run_on_vm '$APP_NAME' 'gh --version'"
        ;;

    ai-toolkit)
        run_test "ollama check" "assert_command ollama '$APP_NAME'"
        run_test "codex check" "assert_command codex '$APP_NAME'"
        ;;

    *)
        log_warning "No specific functionality tests for $EXTENSION"
        ;;
esac

# 4. Idempotency Test
log_info "Phase 4: Idempotency Test"
run_test "idempotency" "test_idempotency '$APP_NAME' '$EXTENSION'"

# 5. File system checks
log_info "Phase 5: File System Checks"

case "$EXTENSION" in
    nodejs|python|golang|rust|ruby)
        run_test "mise config" "assert_file_exists '/workspace/.mise.toml' '$APP_NAME'"
        ;;
    docker)
        run_test "docker socket" "assert_file_exists '/var/run/docker.sock' '$APP_NAME'"
        ;;
    *)
        log_info "No specific file system checks for $EXTENSION"
        ;;
esac

# 6. Environment variable checks
log_info "Phase 6: Environment Checks"

case "$EXTENSION" in
    nodejs)
        run_test "NODE_ENV" "run_on_vm '$APP_NAME' 'echo \$NODE_ENV'"
        ;;
    python)
        run_test "PYTHONPATH" "run_on_vm '$APP_NAME' 'echo \$PYTHONPATH'"
        ;;
    golang)
        run_test "GOPATH" "run_on_vm '$APP_NAME' 'echo \$GOPATH'"
        ;;
    *)
        log_info "No specific environment checks for $EXTENSION"
        ;;
esac

# Summary
echo ""
log_info "Test Summary for $EXTENSION:"
echo "  Passed: $TESTS_PASSED"
echo "  Failed: $TESTS_FAILED"

if [[ $TESTS_FAILED -eq 0 ]]; then
    log_success "All tests passed!"
    exit 0
else
    log_error "Some tests failed"
    exit 1
fi