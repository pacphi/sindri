#!/usr/bin/env bash
# test/unit/yaml/run-all-yaml-tests.sh
# Master runner for all YAML validation tests
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m'

FAILURES=0
PASSED=0

run_test() {
  local test_name="$1"
  local test_script="$2"

  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo -e "${BLUE}Running: $test_name${NC}"
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo ""

  if "$test_script"; then
    echo ""
    echo -e "${GREEN}✓ $test_name PASSED${NC}"
    ((PASSED++))
  else
    echo ""
    echo -e "${RED}✗ $test_name FAILED${NC}"
    ((FAILURES++))
  fi
  echo ""
}

echo ""
echo -e "${BLUE}╔══════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║           Sindri YAML Validation Test Suite              ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════╝${NC}"
echo ""

# Core schema validation tests
run_test "Extension Schema Validation" "$SCRIPT_DIR/test-extension-schemas.sh"
run_test "Profile Schema Validation" "$SCRIPT_DIR/test-profile-schema.sh"
run_test "Registry Schema Validation" "$SCRIPT_DIR/test-registry-schema.sh"
run_test "Categories Schema Validation" "$SCRIPT_DIR/test-categories-schema.sh"
run_test "Templates Schema Validation" "$SCRIPT_DIR/test-templates-schema.sh"
run_test "Sindri Examples Validation" "$SCRIPT_DIR/test-sindri-examples.sh"

# Cross-reference validation
run_test "Cross-Reference Validation" "$SCRIPT_DIR/test-cross-references.sh"

# Quality checks
run_test "Extension Completeness" "$SCRIPT_DIR/test-extension-completeness.sh"
run_test "Profile Dependencies" "$SCRIPT_DIR/test-profile-dependencies.sh"
run_test "Description Quality" "$SCRIPT_DIR/test-description-quality.sh"
run_test "Naming Consistency" "$SCRIPT_DIR/test-naming-consistency.sh"

# YAML lint (optional - may have many warnings)
if command -v yamllint &> /dev/null; then
  run_test "YAML Lint" "$SCRIPT_DIR/test-yaml-lint.sh" || true
else
  echo -e "${YELLOW}Skipping YAML lint (yamllint not installed)${NC}"
fi

# Summary
echo ""
echo -e "${BLUE}╔══════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                    Test Summary                          ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════╝${NC}"
echo ""
echo "  Passed: $PASSED"
echo "  Failed: $FAILURES"
echo "  Total:  $((PASSED + FAILURES))"
echo ""

if [[ $FAILURES -gt 0 ]]; then
  echo -e "${RED}Some tests failed!${NC}"
  exit 1
else
  echo -e "${GREEN}All tests passed!${NC}"
fi
