#!/usr/bin/env bash
# test/e2b/run-tests.sh
# Test runner for E2B provider adapter tests
#
# Usage:
#   ./run-tests.sh [OPTIONS] [SUITE]
#
# Suites:
#   unit         Run unit tests only (no external dependencies)
#   integration  Run integration tests (may require E2B CLI)
#   e2e          Run end-to-end tests (requires E2B_API_KEY)
#   schema       Run schema validation tests
#   smoke        Quick smoke test (subset of unit tests)
#   all          Run all tests (default)
#
# Options:
#   --verbose    Show detailed output
#   --ci         CI mode (fail fast, no interactive prompts)
#   --help       Show this help message
#
# Environment:
#   E2B_API_KEY  Required for integration and e2e tests
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Default options
VERBOSE=false
CI_MODE=false
TEST_SUITE="all"

# ============================================================================
# Utility Functions
# ============================================================================

show_help() {
  cat <<EOF
E2B Provider Adapter Test Runner

Usage:
  ./run-tests.sh [OPTIONS] [SUITE]

Suites:
  unit         Run unit tests only (no external dependencies)
  integration  Run integration tests (may require E2B CLI)
  e2e          Run end-to-end tests (requires E2B_API_KEY)
  schema       Run schema validation tests
  smoke        Quick smoke test (subset of unit tests)
  all          Run all tests (default)

Options:
  --verbose    Show detailed output
  --ci         CI mode (fail fast, no interactive prompts)
  --help       Show this help message

Environment Variables:
  E2B_API_KEY  API key for E2B (required for integration/e2e tests)

Examples:
  ./run-tests.sh                    # Run all tests
  ./run-tests.sh unit               # Run unit tests only
  ./run-tests.sh --ci integration   # Run integration tests in CI mode
  ./run-tests.sh --verbose e2e      # Run E2E tests with verbose output

Exit Codes:
  0  All tests passed
  1  Some tests failed
  2  Missing dependencies or configuration error
EOF
}

log_info() {
  echo -e "${BLUE}[INFO]${NC} $*"
}

log_success() {
  echo -e "${GREEN}[SUCCESS]${NC} $*"
}

log_warn() {
  echo -e "${YELLOW}[WARN]${NC} $*"
}

log_error() {
  echo -e "${RED}[ERROR]${NC} $*"
}

# ============================================================================
# Dependency Checks
# ============================================================================

check_dependencies() {
  local missing=()

  # Required for all tests
  if ! command -v bash &>/dev/null; then
    missing+=("bash")
  fi

  if ! command -v yq &>/dev/null; then
    missing+=("yq")
  fi

  # Optional but recommended
  if ! command -v jq &>/dev/null; then
    log_warn "jq not found - some tests may be skipped"
  fi

  if [[ ${#missing[@]} -gt 0 ]]; then
    log_error "Missing required dependencies: ${missing[*]}"
    echo ""
    echo "Install with:"
    echo "  brew install yq jq  # macOS"
    echo "  pip install yq && apt-get install jq  # Linux"
    return 2
  fi

  return 0
}

check_e2b_cli() {
  if command -v e2b &>/dev/null; then
    local version
    version=$(e2b --version 2>&1 | head -1 || echo "unknown")
    log_info "E2B CLI found: $version"
    return 0
  else
    log_warn "E2B CLI not installed"
    echo "  Install with: npm install -g @e2b/cli"
    return 1
  fi
}

check_e2b_api_key() {
  if [[ -n "${E2B_API_KEY:-}" ]]; then
    log_info "E2B_API_KEY is set"
    return 0
  else
    log_warn "E2B_API_KEY not set - integration/e2e tests will be skipped"
    return 1
  fi
}

# ============================================================================
# Test Suite Runners
# ============================================================================

run_smoke_tests() {
  log_info "Running smoke tests..."

  local exit_code=0

  # Quick sanity checks
  cd "$PROJECT_ROOT"

  # Check test script exists
  if [[ ! -f "$SCRIPT_DIR/test-e2b-adapter.sh" ]]; then
    log_error "Test script not found: $SCRIPT_DIR/test-e2b-adapter.sh"
    return 1
  fi

  # Run minimal unit tests
  "$SCRIPT_DIR/test-e2b-adapter.sh" unit || exit_code=$?

  return $exit_code
}

run_unit_tests() {
  log_info "Running unit tests..."

  local exit_code=0
  "$SCRIPT_DIR/test-e2b-adapter.sh" unit || exit_code=$?
  return $exit_code
}

run_integration_tests() {
  log_info "Running integration tests..."

  # Check prerequisites
  local can_run=true

  if ! check_e2b_cli; then
    can_run=false
  fi

  if [[ "$can_run" == "false" ]]; then
    log_warn "Skipping integration tests due to missing dependencies"
    return 0
  fi

  local exit_code=0
  "$SCRIPT_DIR/test-e2b-adapter.sh" integration || exit_code=$?
  return $exit_code
}

run_e2e_tests() {
  log_info "Running end-to-end tests..."

  # Check prerequisites
  local can_run=true

  if ! check_e2b_cli; then
    can_run=false
  fi

  if ! check_e2b_api_key; then
    can_run=false
  fi

  if [[ "$can_run" == "false" ]]; then
    log_warn "Skipping E2E tests due to missing dependencies or API key"
    return 0
  fi

  local exit_code=0
  "$SCRIPT_DIR/test-e2b-adapter.sh" e2e || exit_code=$?
  return $exit_code
}

run_schema_tests() {
  log_info "Running schema validation tests..."

  local exit_code=0
  "$SCRIPT_DIR/test-e2b-adapter.sh" schema || exit_code=$?
  return $exit_code
}

run_all_tests() {
  local exit_code=0
  local failed_suites=()

  run_unit_tests || {
    exit_code=1
    failed_suites+=("unit")
  }

  run_schema_tests || {
    exit_code=1
    failed_suites+=("schema")
  }

  run_integration_tests || {
    exit_code=1
    failed_suites+=("integration")
  }

  run_e2e_tests || {
    exit_code=1
    failed_suites+=("e2e")
  }

  if [[ ${#failed_suites[@]} -gt 0 ]]; then
    log_error "Failed test suites: ${failed_suites[*]}"
  fi

  return $exit_code
}

# ============================================================================
# CI/CD Helpers
# ============================================================================

setup_ci_environment() {
  # Set stricter error handling for CI
  set -euo pipefail

  # Ensure clean state
  export TERM="${TERM:-dumb}"

  # Disable any interactive prompts
  export NONINTERACTIVE=1

  log_info "CI mode enabled"
}

cleanup_e2b_sandboxes() {
  # Clean up any test sandboxes that may have been left behind
  if command -v e2b &>/dev/null && [[ -n "${E2B_API_KEY:-}" ]]; then
    log_info "Cleaning up test sandboxes..."

    local sandboxes
    sandboxes=$(e2b sandbox list --json 2>/dev/null | jq -r '.[] | select(.metadata.project == "test-project" or (.sandboxId | startswith("test-"))) | .sandboxId' 2>/dev/null || echo "")

    for sandbox_id in $sandboxes; do
      if [[ -n "$sandbox_id" ]]; then
        log_info "Killing sandbox: $sandbox_id"
        e2b sandbox kill "$sandbox_id" 2>/dev/null || true
      fi
    done
  fi
}

# ============================================================================
# Main Entry Point
# ============================================================================

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
    --verbose | -v)
      VERBOSE=true
      shift
      ;;
    --ci)
      CI_MODE=true
      shift
      ;;
    --help | -h)
      show_help
      exit 0
      ;;
    unit | integration | e2e | schema | smoke | all)
      TEST_SUITE="$1"
      shift
      ;;
    *)
      log_error "Unknown option: $1"
      show_help
      exit 2
      ;;
    esac
  done
}

main() {
  parse_args "$@"

  echo ""
  echo -e "${BLUE}╔══════════════════════════════════════════════════════════╗${NC}"
  echo -e "${BLUE}║         E2B Provider Adapter - Test Runner               ║${NC}"
  echo -e "${BLUE}╚══════════════════════════════════════════════════════════╝${NC}"
  echo ""

  # Setup
  if [[ "$CI_MODE" == "true" ]]; then
    setup_ci_environment
  fi

  # Check dependencies
  check_dependencies || exit 2

  # Make test script executable
  chmod +x "$SCRIPT_DIR/test-e2b-adapter.sh" 2>/dev/null || true

  # Run selected test suite
  local exit_code=0

  case "$TEST_SUITE" in
  unit)
    run_unit_tests || exit_code=$?
    ;;
  integration)
    run_integration_tests || exit_code=$?
    ;;
  e2e)
    run_e2e_tests || exit_code=$?
    ;;
  schema)
    run_schema_tests || exit_code=$?
    ;;
  smoke)
    run_smoke_tests || exit_code=$?
    ;;
  all)
    run_all_tests || exit_code=$?
    ;;
  esac

  # Cleanup in CI mode
  if [[ "$CI_MODE" == "true" ]]; then
    cleanup_e2b_sandboxes
  fi

  # Final result
  echo ""
  if [[ $exit_code -eq 0 ]]; then
    log_success "All tests completed successfully!"
  else
    log_error "Some tests failed"
  fi

  exit $exit_code
}

# Run if executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  main "$@"
fi
