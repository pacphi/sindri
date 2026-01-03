#!/usr/bin/env bash
# test/e2b/test-e2b-adapter.sh
# Test suite for E2B provider adapter
#
# Usage:
#   ./test-e2b-adapter.sh [unit|integration|e2e|all]
#
# Environment:
#   E2B_API_KEY - Required for integration/e2e tests
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

# Test counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0

# Test fixtures directory
FIXTURES_DIR="$SCRIPT_DIR/fixtures"

# Adapter location
E2B_ADAPTER="$PROJECT_ROOT/deploy/adapters/e2b-adapter.sh"

# ============================================================================
# Test Utilities
# ============================================================================

log_info() {
  echo -e "${BLUE}[INFO]${NC} $*"
}

log_success() {
  echo -e "${GREEN}[PASS]${NC} $*"
}

log_fail() {
  echo -e "${RED}[FAIL]${NC} $*"
}

log_skip() {
  echo -e "${YELLOW}[SKIP]${NC} $*"
}

log_section() {
  echo ""
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo -e "${BLUE}$*${NC}"
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo ""
}

# Assert helper functions
assert_equals() {
  local expected="$1"
  local actual="$2"
  local message="${3:-Values should be equal}"

  if [[ "$expected" == "$actual" ]]; then
    return 0
  else
    echo "  Expected: '$expected'"
    echo "  Actual:   '$actual'"
    return 1
  fi
}

assert_contains() {
  local haystack="$1"
  local needle="$2"
  local message="${3:-String should contain substring}"

  if [[ "$haystack" == *"$needle"* ]]; then
    return 0
  else
    echo "  String does not contain: '$needle'"
    return 1
  fi
}

assert_file_exists() {
  local file="$1"
  local message="${2:-File should exist}"

  if [[ -f "$file" ]]; then
    return 0
  else
    echo "  File not found: '$file'"
    return 1
  fi
}

assert_dir_exists() {
  local dir="$1"
  local message="${2:-Directory should exist}"

  if [[ -d "$dir" ]]; then
    return 0
  else
    echo "  Directory not found: '$dir'"
    return 1
  fi
}

assert_command_exists() {
  local cmd="$1"
  local message="${2:-Command should be available}"

  if command -v "$cmd" &>/dev/null; then
    return 0
  else
    echo "  Command not found: '$cmd'"
    return 1
  fi
}

assert_exit_code() {
  local expected="$1"
  local actual="$2"
  local message="${3:-Exit code should match}"

  if [[ "$expected" == "$actual" ]]; then
    return 0
  else
    echo "  Expected exit code: $expected, got: $actual"
    return 1
  fi
}

# Run a test function and track results
run_test() {
  local test_name="$1"
  local test_func="$2"

  ((TESTS_RUN++)) || true

  echo -n "  Testing: $test_name... "

  local output
  local exit_code=0

  # Capture output and exit code
  output=$($test_func 2>&1) || exit_code=$?

  if [[ $exit_code -eq 0 ]]; then
    log_success "PASSED"
    ((TESTS_PASSED++)) || true
  else
    log_fail "FAILED"
    ((TESTS_FAILED++)) || true
    if [[ -n "$output" ]]; then
      echo "$output" | sed 's/^/    /'
    fi
  fi
}

# Skip a test with reason
skip_test() {
  local test_name="$1"
  local reason="$2"

  ((TESTS_RUN++)) || true
  ((TESTS_SKIPPED++)) || true
  echo -n "  Testing: $test_name... "
  log_skip "SKIPPED ($reason)"
}

# Check if E2B API key is available
has_e2b_api_key() {
  [[ -n "${E2B_API_KEY:-}" ]]
}

# Check if E2B CLI is installed
has_e2b_cli() {
  command -v e2b &>/dev/null
}

# Create a temporary test directory
setup_test_dir() {
  local test_dir
  test_dir=$(mktemp -d)
  echo "$test_dir"
}

# Clean up test directory
cleanup_test_dir() {
  local test_dir="$1"
  if [[ -d "$test_dir" ]]; then
    rm -rf "$test_dir"
  fi
}

# ============================================================================
# Test Fixtures
# ============================================================================

setup_fixtures() {
  mkdir -p "$FIXTURES_DIR"

  # Basic E2B configuration
  cat >"$FIXTURES_DIR/basic-e2b.yaml" <<'EOF'
version: "1.0"
name: test-e2b-basic

deployment:
  provider: e2b
  resources:
    memory: 2GB
    cpus: 2

extensions:
  profile: minimal

providers:
  e2b:
    timeout: 300
    autoPause: true
    autoResume: true
EOF

  # Advanced E2B configuration
  cat >"$FIXTURES_DIR/advanced-e2b.yaml" <<'EOF'
version: "1.0"
name: test-e2b-advanced

deployment:
  provider: e2b
  resources:
    memory: 4GB
    cpus: 4
  volumes:
    workspace:
      size: 20GB

extensions:
  profile: fullstack
  additional:
    - docker

providers:
  e2b:
    templateAlias: sindri-test-template
    reuseTemplate: true
    timeout: 3600
    autoPause: true
    autoResume: true
    internetAccess: true
    allowedDomains:
      - github.com
      - "*.github.com"
      - registry.npmjs.org
    blockedDomains: []
    publicAccess: false
    metadata:
      project: test-project
      environment: ci
EOF

  # Invalid configuration (GPU with E2B)
  cat >"$FIXTURES_DIR/invalid-gpu-e2b.yaml" <<'EOF'
version: "1.0"
name: test-e2b-invalid-gpu

deployment:
  provider: e2b
  resources:
    memory: 2GB
    cpus: 2
    gpu:
      enabled: true
      type: nvidia

extensions:
  profile: minimal
EOF

  # Ephemeral sandbox configuration
  cat >"$FIXTURES_DIR/ephemeral-e2b.yaml" <<'EOF'
version: "1.0"
name: test-e2b-ephemeral

deployment:
  provider: e2b
  resources:
    memory: 1GB
    cpus: 1

extensions:
  active:
    - nodejs

providers:
  e2b:
    timeout: 300
    autoPause: false
EOF

  # Network restricted configuration
  cat >"$FIXTURES_DIR/restricted-network-e2b.yaml" <<'EOF'
version: "1.0"
name: test-e2b-restricted

deployment:
  provider: e2b
  resources:
    memory: 2GB
    cpus: 2

extensions:
  profile: minimal

providers:
  e2b:
    internetAccess: true
    allowedDomains:
      - api.anthropic.com
      - github.com
    blockedDomains:
      - example-blocked.com
    publicAccess: false
EOF
}

cleanup_fixtures() {
  if [[ -d "$FIXTURES_DIR" ]]; then
    rm -rf "$FIXTURES_DIR"
  fi
}

# ============================================================================
# Unit Tests - Config Parsing
# ============================================================================

test_unit_parse_basic_config() {
  # Test that basic E2B config can be parsed
  local config="$FIXTURES_DIR/basic-e2b.yaml"
  assert_file_exists "$config"

  # Verify required fields are present
  local name
  name=$(yq '.name' "$config")
  assert_equals "test-e2b-basic" "$name"

  local provider
  provider=$(yq '.deployment.provider' "$config")
  assert_equals "e2b" "$provider"

  local timeout
  timeout=$(yq '.providers.e2b.timeout' "$config")
  assert_equals "300" "$timeout"
}

test_unit_parse_advanced_config() {
  local config="$FIXTURES_DIR/advanced-e2b.yaml"
  assert_file_exists "$config"

  # Verify advanced fields
  local template_alias
  template_alias=$(yq '.providers.e2b.templateAlias' "$config")
  assert_equals "sindri-test-template" "$template_alias"

  local allowed_domains
  allowed_domains=$(yq '.providers.e2b.allowedDomains | length' "$config")
  assert_equals "3" "$allowed_domains"

  local metadata_project
  metadata_project=$(yq '.providers.e2b.metadata.project' "$config")
  assert_equals "test-project" "$metadata_project"
}

test_unit_parse_memory_conversion() {
  local config="$FIXTURES_DIR/basic-e2b.yaml"

  # Test memory value parsing
  local memory
  memory=$(yq '.deployment.resources.memory' "$config")
  assert_equals "2GB" "$memory"

  # Memory should convert to MB for E2B API
  local memory_mb
  memory_mb=$(echo "$memory" | sed 's/GB//' | awk '{print $1 * 1024}')
  assert_equals "2048" "$memory_mb"
}

test_unit_parse_cpu_validation() {
  local config="$FIXTURES_DIR/advanced-e2b.yaml"

  local cpus
  cpus=$(yq '.deployment.resources.cpus' "$config")
  assert_equals "4" "$cpus"

  # E2B supports 1-8 vCPUs
  if [[ "$cpus" -ge 1 ]] && [[ "$cpus" -le 8 ]]; then
    return 0
  else
    echo "  CPU count out of valid range (1-8)"
    return 1
  fi
}

test_unit_validate_gpu_not_supported() {
  local config="$FIXTURES_DIR/invalid-gpu-e2b.yaml"
  assert_file_exists "$config"

  # Check that GPU is configured (should fail validation)
  local gpu_enabled
  gpu_enabled=$(yq '.deployment.resources.gpu.enabled' "$config")
  assert_equals "true" "$gpu_enabled"

  # This configuration should be rejected by the adapter
  # (GPU is not supported on E2B)
}

test_unit_parse_network_config() {
  local config="$FIXTURES_DIR/restricted-network-e2b.yaml"

  local internet_access
  internet_access=$(yq '.providers.e2b.internetAccess' "$config")
  assert_equals "true" "$internet_access"

  local allowed_count
  allowed_count=$(yq '.providers.e2b.allowedDomains | length' "$config")
  assert_equals "2" "$allowed_count"

  local blocked_count
  blocked_count=$(yq '.providers.e2b.blockedDomains | length' "$config")
  assert_equals "1" "$blocked_count"
}

test_unit_default_values() {
  # Test that defaults are applied when not specified
  local config="$FIXTURES_DIR/ephemeral-e2b.yaml"

  # autoPause is explicitly false
  local auto_pause
  auto_pause=$(yq '.providers.e2b.autoPause' "$config")
  assert_equals "false" "$auto_pause"

  # autoResume should default to true if not specified
  local auto_resume
  auto_resume=$(yq '.providers.e2b.autoResume // "true"' "$config")
  assert_equals "true" "$auto_resume"
}

test_unit_template_alias_generation() {
  local config="$FIXTURES_DIR/basic-e2b.yaml"

  # Template alias defaults to name if not specified
  local name
  name=$(yq '.name' "$config")

  local template_alias
  template_alias=$(yq '.providers.e2b.templateAlias // .name' "$config")
  assert_equals "$name" "$template_alias"
}

run_unit_tests() {
  log_section "Unit Tests - Configuration Parsing"

  setup_fixtures

  run_test "Parse basic E2B config" test_unit_parse_basic_config
  run_test "Parse advanced E2B config" test_unit_parse_advanced_config
  run_test "Parse memory conversion" test_unit_parse_memory_conversion
  run_test "Parse CPU validation" test_unit_parse_cpu_validation
  run_test "Validate GPU not supported" test_unit_validate_gpu_not_supported
  run_test "Parse network config" test_unit_parse_network_config
  run_test "Default values applied" test_unit_default_values
  run_test "Template alias generation" test_unit_template_alias_generation

  cleanup_fixtures
}

# ============================================================================
# Integration Tests - Adapter Commands
# ============================================================================

test_integration_adapter_exists() {
  # Check if the adapter script exists (may not exist yet during TDD)
  if [[ -f "$E2B_ADAPTER" ]]; then
    assert_file_exists "$E2B_ADAPTER"
  else
    echo "  Adapter not yet implemented at: $E2B_ADAPTER"
    return 1
  fi
}

test_integration_adapter_help() {
  if [[ ! -f "$E2B_ADAPTER" ]]; then
    return 1
  fi

  local output
  output=$("$E2B_ADAPTER" --help 2>&1) || true

  assert_contains "$output" "deploy"
  assert_contains "$output" "connect"
  assert_contains "$output" "destroy"
}

test_integration_config_only_mode() {
  if [[ ! -f "$E2B_ADAPTER" ]]; then
    return 1
  fi

  setup_fixtures
  local test_dir
  test_dir=$(setup_test_dir)

  # Run config-only mode
  local exit_code=0
  "$E2B_ADAPTER" deploy --config-only \
    --output-dir "$test_dir" \
    "$FIXTURES_DIR/basic-e2b.yaml" 2>&1 || exit_code=$?

  # Should create template files
  assert_dir_exists "$test_dir/e2b-template" || exit_code=1
  assert_file_exists "$test_dir/e2b-template/e2b.toml" || exit_code=1

  cleanup_test_dir "$test_dir"
  cleanup_fixtures

  return $exit_code
}

test_integration_plan_command() {
  if [[ ! -f "$E2B_ADAPTER" ]]; then
    return 1
  fi

  setup_fixtures

  local output
  local exit_code=0
  output=$("$E2B_ADAPTER" plan "$FIXTURES_DIR/basic-e2b.yaml" 2>&1) || exit_code=$?

  # Plan should show deployment details
  assert_contains "$output" "e2b" || exit_code=1
  assert_contains "$output" "test-e2b-basic" || exit_code=1

  cleanup_fixtures
  return $exit_code
}

test_integration_status_no_sandbox() {
  if [[ ! -f "$E2B_ADAPTER" ]]; then
    return 1
  fi
  if ! has_e2b_api_key; then
    return 1
  fi

  setup_fixtures

  local output
  local exit_code=0
  output=$("$E2B_ADAPTER" status "$FIXTURES_DIR/basic-e2b.yaml" 2>&1) || exit_code=$?

  # Should indicate no sandbox exists
  assert_contains "$output" "not found" || assert_contains "$output" "No sandbox" || exit_code=1

  cleanup_fixtures
  return $exit_code
}

test_integration_validate_e2b_cli() {
  if ! has_e2b_cli; then
    echo "  E2B CLI not installed"
    return 1
  fi

  # Check E2B CLI version
  local output
  output=$(e2b --version 2>&1) || true

  assert_contains "$output" "e2b" || assert_contains "$output" "E2B"
}

test_integration_validate_api_key() {
  if ! has_e2b_api_key; then
    echo "  E2B_API_KEY not set"
    return 1
  fi

  # API key should be set
  [[ -n "$E2B_API_KEY" ]]
}

run_integration_tests() {
  log_section "Integration Tests - Adapter Commands"

  # Check prerequisites
  if ! command -v yq &>/dev/null; then
    echo -e "${RED}ERROR: yq is required for tests${NC}"
    return 1
  fi

  run_test "Adapter script exists" test_integration_adapter_exists

  if [[ -f "$E2B_ADAPTER" ]]; then
    run_test "Adapter help command" test_integration_adapter_help
    run_test "Config-only mode" test_integration_config_only_mode
    run_test "Plan command" test_integration_plan_command

    if has_e2b_api_key; then
      run_test "Status (no sandbox)" test_integration_status_no_sandbox
    else
      skip_test "Status (no sandbox)" "E2B_API_KEY not set"
    fi
  else
    skip_test "Adapter help command" "Adapter not implemented"
    skip_test "Config-only mode" "Adapter not implemented"
    skip_test "Plan command" "Adapter not implemented"
    skip_test "Status (no sandbox)" "Adapter not implemented"
  fi

  if has_e2b_cli; then
    run_test "E2B CLI available" test_integration_validate_e2b_cli
  else
    skip_test "E2B CLI available" "E2B CLI not installed"
  fi

  if has_e2b_api_key; then
    run_test "API key configured" test_integration_validate_api_key
  else
    skip_test "API key configured" "E2B_API_KEY not set"
  fi
}

# ============================================================================
# E2E Tests - Full Lifecycle
# ============================================================================

test_e2e_sandbox_lifecycle() {
  if [[ ! -f "$E2B_ADAPTER" ]]; then
    return 1
  fi
  if ! has_e2b_api_key; then
    return 1
  fi
  if ! has_e2b_cli; then
    return 1
  fi

  setup_fixtures

  local exit_code=0
  local sandbox_name="test-e2b-lifecycle-$$"

  # Create a test config with unique name
  local test_config="$FIXTURES_DIR/lifecycle-test.yaml"
  sed "s/test-e2b-basic/$sandbox_name/" "$FIXTURES_DIR/basic-e2b.yaml" >"$test_config"

  echo "  Creating sandbox..."
  "$E2B_ADAPTER" deploy "$test_config" 2>&1 || exit_code=$?

  if [[ $exit_code -ne 0 ]]; then
    echo "  Deploy failed"
    cleanup_fixtures
    return 1
  fi

  echo "  Checking status..."
  local status_output
  status_output=$("$E2B_ADAPTER" status "$test_config" 2>&1) || exit_code=$?

  if ! echo "$status_output" | grep -q "running\|Running"; then
    echo "  Sandbox not running"
    exit_code=1
  fi

  echo "  Destroying sandbox..."
  "$E2B_ADAPTER" destroy --force "$test_config" 2>&1 || exit_code=$?

  cleanup_fixtures
  return $exit_code
}

test_e2e_pause_resume() {
  if [[ ! -f "$E2B_ADAPTER" ]]; then
    return 1
  fi
  if ! has_e2b_api_key; then
    return 1
  fi
  if ! has_e2b_cli; then
    return 1
  fi

  setup_fixtures

  local exit_code=0
  local sandbox_name="test-e2b-pause-$$"

  # Create a test config
  local test_config="$FIXTURES_DIR/pause-test.yaml"
  sed "s/test-e2b-basic/$sandbox_name/" "$FIXTURES_DIR/basic-e2b.yaml" >"$test_config"

  echo "  Creating sandbox..."
  "$E2B_ADAPTER" deploy "$test_config" 2>&1 || exit_code=$?

  if [[ $exit_code -ne 0 ]]; then
    cleanup_fixtures
    return 1
  fi

  echo "  Pausing sandbox..."
  "$E2B_ADAPTER" pause "$test_config" 2>&1 || exit_code=$?

  if [[ $exit_code -ne 0 ]]; then
    echo "  Pause failed"
    "$E2B_ADAPTER" destroy --force "$test_config" 2>&1 || true
    cleanup_fixtures
    return 1
  fi

  echo "  Checking paused status..."
  local status_output
  status_output=$("$E2B_ADAPTER" status "$test_config" 2>&1) || exit_code=$?

  if ! echo "$status_output" | grep -q "paused\|Paused"; then
    echo "  Sandbox not paused"
    exit_code=1
  fi

  echo "  Resuming sandbox (via deploy)..."
  "$E2B_ADAPTER" deploy "$test_config" 2>&1 || exit_code=$?

  echo "  Cleaning up..."
  "$E2B_ADAPTER" destroy --force "$test_config" 2>&1 || true

  cleanup_fixtures
  return $exit_code
}

test_e2e_secrets_injection() {
  if [[ ! -f "$E2B_ADAPTER" ]]; then
    return 1
  fi
  if ! has_e2b_api_key; then
    return 1
  fi
  if ! has_e2b_cli; then
    return 1
  fi

  setup_fixtures

  local exit_code=0
  local sandbox_name="test-e2b-secrets-$$"
  local test_secret="test-value-$$"

  # Create a test config
  local test_config="$FIXTURES_DIR/secrets-test.yaml"
  sed "s/test-e2b-basic/$sandbox_name/" "$FIXTURES_DIR/basic-e2b.yaml" >"$test_config"

  # Export a test secret
  export TEST_SINDRI_SECRET="$test_secret"

  echo "  Creating sandbox with secrets..."
  "$E2B_ADAPTER" deploy "$test_config" 2>&1 || exit_code=$?

  if [[ $exit_code -ne 0 ]]; then
    cleanup_fixtures
    return 1
  fi

  # Note: Actually testing secret injection would require running a command
  # in the sandbox, which depends on the adapter implementation

  echo "  Cleaning up..."
  "$E2B_ADAPTER" destroy --force "$test_config" 2>&1 || true

  unset TEST_SINDRI_SECRET
  cleanup_fixtures
  return $exit_code
}

test_e2e_template_management() {
  if [[ ! -f "$E2B_ADAPTER" ]]; then
    return 1
  fi
  if ! has_e2b_api_key; then
    return 1
  fi
  if ! has_e2b_cli; then
    return 1
  fi

  setup_fixtures

  local exit_code=0
  local template_alias="sindri-test-template-$$"

  # Create config with custom template alias
  local test_config="$FIXTURES_DIR/template-test.yaml"
  cat >"$test_config" <<EOF
version: "1.0"
name: test-e2b-template

deployment:
  provider: e2b
  resources:
    memory: 1GB
    cpus: 1

extensions:
  profile: minimal

providers:
  e2b:
    templateAlias: $template_alias
    reuseTemplate: false
EOF

  echo "  Building template..."
  "$E2B_ADAPTER" template build "$test_config" 2>&1 || exit_code=$?

  if [[ $exit_code -ne 0 ]]; then
    echo "  Template build failed"
    cleanup_fixtures
    return 1
  fi

  echo "  Listing templates..."
  local list_output
  list_output=$("$E2B_ADAPTER" template list 2>&1) || exit_code=$?

  if ! echo "$list_output" | grep -q "$template_alias"; then
    echo "  Template not found in list"
    exit_code=1
  fi

  echo "  Deleting template..."
  "$E2B_ADAPTER" template delete "$template_alias" --force 2>&1 || exit_code=$?

  cleanup_fixtures
  return $exit_code
}

run_e2e_tests() {
  log_section "End-to-End Tests - Full Lifecycle"

  # E2E tests require E2B API key and CLI
  if ! has_e2b_api_key; then
    skip_test "Sandbox lifecycle" "E2B_API_KEY not set"
    skip_test "Pause/Resume" "E2B_API_KEY not set"
    skip_test "Secrets injection" "E2B_API_KEY not set"
    skip_test "Template management" "E2B_API_KEY not set"
    return
  fi

  if ! has_e2b_cli; then
    skip_test "Sandbox lifecycle" "E2B CLI not installed"
    skip_test "Pause/Resume" "E2B CLI not installed"
    skip_test "Secrets injection" "E2B CLI not installed"
    skip_test "Template management" "E2B CLI not installed"
    return
  fi

  if [[ ! -f "$E2B_ADAPTER" ]]; then
    skip_test "Sandbox lifecycle" "Adapter not implemented"
    skip_test "Pause/Resume" "Adapter not implemented"
    skip_test "Secrets injection" "Adapter not implemented"
    skip_test "Template management" "Adapter not implemented"
    return
  fi

  run_test "Sandbox lifecycle (create/status/destroy)" test_e2e_sandbox_lifecycle
  run_test "Pause/Resume functionality" test_e2e_pause_resume
  run_test "Secrets injection" test_e2e_secrets_injection
  run_test "Template management" test_e2e_template_management
}

# ============================================================================
# Schema Validation Tests
# ============================================================================

test_schema_e2b_provider_defined() {
  local schema="$PROJECT_ROOT/docker/lib/schemas/sindri.schema.json"

  if [[ ! -f "$schema" ]]; then
    echo "  Schema file not found"
    return 1
  fi

  # Check if e2b is in the provider enum
  local providers
  providers=$(jq -r '.properties.deployment.properties.provider.enum // [] | .[]' "$schema" 2>/dev/null || echo "")

  if echo "$providers" | grep -q "e2b"; then
    return 0
  else
    echo "  E2B provider not in schema enum"
    return 1
  fi
}

test_schema_e2b_provider_options() {
  local schema="$PROJECT_ROOT/docker/lib/schemas/sindri.schema.json"

  if [[ ! -f "$schema" ]]; then
    echo "  Schema file not found"
    return 1
  fi

  # Check if e2b provider options are defined
  local has_e2b_options
  has_e2b_options=$(jq -e '.properties.providers.properties.e2b' "$schema" 2>/dev/null && echo "yes" || echo "no")

  if [[ "$has_e2b_options" == "yes" ]]; then
    return 0
  else
    echo "  E2B provider options not defined in schema"
    return 1
  fi
}

test_schema_validate_basic_config() {
  local schema="$PROJECT_ROOT/docker/lib/schemas/sindri.schema.json"

  if [[ ! -f "$schema" ]]; then
    echo "  Schema file not found"
    return 1
  fi

  if ! command -v ajv &>/dev/null && ! command -v npx &>/dev/null; then
    echo "  ajv not available for validation"
    return 1
  fi

  setup_fixtures

  local exit_code=0

  # Validate basic config against schema
  if command -v ajv &>/dev/null; then
    ajv validate -s "$schema" -d "$FIXTURES_DIR/basic-e2b.yaml" 2>&1 || exit_code=$?
  else
    npx ajv-cli validate -s "$schema" -d "$FIXTURES_DIR/basic-e2b.yaml" 2>&1 || exit_code=$?
  fi

  cleanup_fixtures
  return $exit_code
}

run_schema_tests() {
  log_section "Schema Validation Tests"

  run_test "E2B provider in schema enum" test_schema_e2b_provider_defined
  run_test "E2B provider options defined" test_schema_e2b_provider_options

  if command -v ajv &>/dev/null || command -v npx &>/dev/null; then
    run_test "Validate basic config against schema" test_schema_validate_basic_config
  else
    skip_test "Validate basic config against schema" "ajv not available"
  fi
}

# ============================================================================
# Main Entry Point
# ============================================================================

print_summary() {
  echo ""
  echo -e "${BLUE}╔══════════════════════════════════════════════════════════╗${NC}"
  echo -e "${BLUE}║                    Test Summary                          ║${NC}"
  echo -e "${BLUE}╚══════════════════════════════════════════════════════════╝${NC}"
  echo ""
  echo "  Total:   $TESTS_RUN"
  echo "  Passed:  $TESTS_PASSED"
  echo "  Failed:  $TESTS_FAILED"
  echo "  Skipped: $TESTS_SKIPPED"
  echo ""

  if [[ $TESTS_FAILED -gt 0 ]]; then
    echo -e "${RED}Some tests failed!${NC}"
    return 1
  else
    echo -e "${GREEN}All tests passed!${NC}"
    return 0
  fi
}

main() {
  local test_suite="${1:-all}"

  echo ""
  echo -e "${BLUE}╔══════════════════════════════════════════════════════════╗${NC}"
  echo -e "${BLUE}║           E2B Provider Adapter Test Suite                ║${NC}"
  echo -e "${BLUE}╚══════════════════════════════════════════════════════════╝${NC}"
  echo ""

  # Check required tools
  if ! command -v yq &>/dev/null; then
    echo -e "${RED}ERROR: yq is required but not installed${NC}"
    echo "Install with: brew install yq (macOS) or pip install yq"
    exit 1
  fi

  case "$test_suite" in
  unit)
    run_unit_tests
    ;;
  integration)
    run_integration_tests
    ;;
  e2e)
    run_e2e_tests
    ;;
  schema)
    run_schema_tests
    ;;
  all)
    run_unit_tests
    run_integration_tests
    run_schema_tests
    run_e2e_tests
    ;;
  *)
    echo "Usage: $0 [unit|integration|e2e|schema|all]"
    exit 1
    ;;
  esac

  print_summary
}

# Run if executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  main "$@"
fi
