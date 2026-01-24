#!/usr/bin/env bash
# test/unit/yaml/validate-schema.sh
# Unified YAML schema validation script
# Replaces 7 inline validation blocks in validate-yaml.yml workflow
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
SCHEMAS_DIR="$PROJECT_ROOT/v2/docker/lib/schemas"

# Colors for output (disabled in CI by default)
if [[ -t 1 ]] && [[ "${NO_COLOR:-}" != "1" ]]; then
  RED='\033[0;31m'
  GREEN='\033[0;32m'
  YELLOW='\033[0;33m'
  NC='\033[0m'
else
  RED=''
  GREEN=''
  YELLOW=''
  NC=''
fi

# Global counters
TOTAL_FAILURES=0
TOTAL_PASSED=0

# Global temp file for JSON conversion (ajv requires .json extension)
GLOBAL_TMPFILE=""

#######################################
# Cleanup function for EXIT trap
#######################################
cleanup() {
  if [[ -n "${GLOBAL_TMPFILE:-}" ]] && [[ -f "$GLOBAL_TMPFILE" ]]; then
    rm -f "$GLOBAL_TMPFILE"
  fi
}
trap cleanup EXIT

#######################################
# Check if required tools are available
# Globals: RED, NC
# Returns: 0 if all tools present, 1 otherwise
#######################################
check_tools() {
  local missing=0

  if ! command -v yq &> /dev/null; then
    echo -e "${RED}ERROR: yq is required but not installed${NC}"
    missing=1
  fi

  if ! command -v ajv &> /dev/null; then
    echo -e "${RED}ERROR: ajv-cli is required but not installed${NC}"
    echo "Install with: npm install -g ajv-cli ajv-formats"
    missing=1
  fi

  return $missing
}

#######################################
# Ensure global temp file exists (cross-platform)
# Sets: GLOBAL_TMPFILE
#######################################
ensure_tmpfile() {
  if [[ -n "$GLOBAL_TMPFILE" ]] && [[ -f "$GLOBAL_TMPFILE" ]]; then
    return 0
  fi

  local tmpfile
  if [[ "$(uname)" == "Darwin" ]]; then
    # macOS mktemp syntax
    tmpfile=$(mktemp /tmp/schema-validate.XXXXXX)
    mv "$tmpfile" "${tmpfile}.json"
    GLOBAL_TMPFILE="${tmpfile}.json"
  else
    # Linux mktemp syntax
    GLOBAL_TMPFILE=$(mktemp --suffix=.json)
  fi
}

#######################################
# Validate a single YAML file against a schema
# Arguments:
#   $1 - YAML file path
#   $2 - JSON schema file path
# Returns: 0 on success, 1 on failure
#######################################
validate_single_file() {
  local yaml_file="$1"
  local schema_file="$2"
  local rel_path="${yaml_file#"$PROJECT_ROOT"/}"

  # Ensure we have a temp file for JSON conversion
  ensure_tmpfile

  echo "Validating: $rel_path"

  local yq_error
  if ! yq_error=$(yq -o=json "$yaml_file" 2>&1 > "$GLOBAL_TMPFILE"); then
    echo -e "${RED}FAIL: $rel_path - Invalid YAML syntax${NC}"
    echo "  Error: $yq_error"
    return 1
  fi

  if ! ajv validate -c ajv-formats -s "$schema_file" -d "$GLOBAL_TMPFILE" 2>&1; then
    echo -e "${RED}FAIL: $rel_path - Schema validation failed${NC}"
    return 1
  fi

  echo -e "${GREEN}PASS: $rel_path${NC}"
  return 0
}

#######################################
# Validate multiple YAML files against a schema
# Arguments:
#   $1 - File glob pattern (e.g., "v2/docker/lib/extensions/*/extension.yaml")
#   $2 - JSON schema file path
#   $3 - Description for output (e.g., "extension(s)")
# Returns: 0 if all pass, 1 if any fail
#######################################
validate_multiple_files() {
  local pattern="$1"
  local schema_file="$2"
  local description="${3:-file(s)}"

  local failures=0
  local passed=0

  # Ensure we have a temp file for JSON conversion
  ensure_tmpfile

  # Use nullglob to handle no matches gracefully
  shopt -s nullglob
  local -a files
  # shellcheck disable=SC2206
  files=($PROJECT_ROOT/$pattern)
  shopt -u nullglob

  if [[ ${#files[@]} -eq 0 ]]; then
    echo -e "${YELLOW}WARN: No files found matching pattern: $pattern${NC}"
    return 0
  fi

  echo "Found ${#files[@]} $description to validate"
  echo ""

  for file in "${files[@]}"; do
    local rel_path="${file#"$PROJECT_ROOT"/}"
    echo "Validating: $rel_path"

    local yq_error
    if ! yq_error=$(yq -o=json "$file" 2>&1 > "$GLOBAL_TMPFILE"); then
      echo -e "${RED}FAIL: $rel_path - Invalid YAML syntax${NC}"
      echo "  Error: $yq_error"
      ((failures++)) || true
      continue
    fi

    if ! ajv validate -c ajv-formats -s "$schema_file" -d "$GLOBAL_TMPFILE" 2>&1; then
      echo -e "${RED}FAIL: $rel_path - Schema validation failed${NC}"
      ((failures++)) || true
    else
      echo -e "${GREEN}PASS: $rel_path${NC}"
      ((passed++)) || true
    fi
  done

  echo ""
  echo "Results: $passed passed, $failures failed"

  # Update global counters
  ((TOTAL_PASSED += passed)) || true
  ((TOTAL_FAILURES += failures)) || true

  if [[ $failures -gt 0 ]]; then
    echo -e "${RED}Failed: $failures $description${NC}"
    return 1
  fi

  return 0
}

#######################################
# Main validation runner
# Validates all known schema types
# Returns: 0 if all pass, 1 if any fail
#######################################
run_all_validations() {
  local exit_code=0

  echo "================================"
  echo "Schema Validation Suite"
  echo "================================"
  echo ""

  # 1. Extension schemas
  echo "--- Extension Schemas ---"
  if ! validate_multiple_files \
    "v2/docker/lib/extensions/*/extension.yaml" \
    "$SCHEMAS_DIR/extension.schema.json" \
    "extension(s)"; then
    exit_code=1
  fi
  echo ""

  # 2. Sindri examples
  echo "--- Sindri Examples ---"
  if ! validate_multiple_files \
    "examples/**/*.sindri.yaml" \
    "$SCHEMAS_DIR/sindri.schema.json" \
    "example(s)"; then
    exit_code=1
  fi
  # Note: All sindri.yaml examples are in subdirectories (docker/, fly/, etc.)
  # so examples/*.sindri.yaml pattern is not needed
  echo ""

  # 3. profiles.yaml
  echo "--- Profiles Schema ---"
  if ! validate_single_file \
    "$PROJECT_ROOT/v2/docker/lib/profiles.yaml" \
    "$SCHEMAS_DIR/profiles.schema.json"; then
    exit_code=1
  fi
  echo ""

  # 4. registry.yaml
  echo "--- Registry Schema ---"
  if ! validate_single_file \
    "$PROJECT_ROOT/v2/docker/lib/registry.yaml" \
    "$SCHEMAS_DIR/registry.schema.json"; then
    exit_code=1
  fi
  echo ""

  # 5. categories.yaml
  echo "--- Categories Schema ---"
  if ! validate_single_file \
    "$PROJECT_ROOT/v2/docker/lib/categories.yaml" \
    "$SCHEMAS_DIR/categories.schema.json"; then
    exit_code=1
  fi
  echo ""

  # 6. project-templates.yaml
  echo "--- Project Templates Schema ---"
  if ! validate_single_file \
    "$PROJECT_ROOT/v2/docker/lib/project-templates.yaml" \
    "$SCHEMAS_DIR/project-templates.schema.json"; then
    exit_code=1
  fi
  echo ""

  # 7. vm-sizes.yaml
  echo "--- VM Sizes Schema ---"
  if ! validate_single_file \
    "$PROJECT_ROOT/v2/docker/lib/vm-sizes.yaml" \
    "$SCHEMAS_DIR/vm-sizes.schema.json"; then
    exit_code=1
  fi
  echo ""

  # Summary
  echo "================================"
  echo "Total: $TOTAL_PASSED passed, $TOTAL_FAILURES failed"
  echo "================================"

  return $exit_code
}

#######################################
# Show usage
#######################################
usage() {
  cat << EOF
Usage: $(basename "$0") [OPTIONS] [COMMAND]

Unified YAML schema validation script.

Commands:
  all                    Run all schema validations (default)
  single FILE SCHEMA     Validate single file against schema
  multiple PATTERN SCHEMA [DESC]  Validate files matching pattern

Options:
  -h, --help            Show this help message

Examples:
  $(basename "$0")
  $(basename "$0") all
  $(basename "$0") single v2/docker/lib/profiles.yaml v2/docker/lib/schemas/profiles.schema.json
  $(basename "$0") multiple "v2/docker/lib/extensions/*/extension.yaml" v2/docker/lib/schemas/extension.schema.json "extension(s)"
EOF
}

# Main entry point
main() {
  local command="${1:-all}"

  case "$command" in
    -h|--help)
      usage
      exit 0
      ;;
    all)
      check_tools || exit 1
      run_all_validations
      ;;
    single)
      check_tools || exit 1
      if [[ $# -lt 3 ]]; then
        echo "Error: 'single' requires FILE and SCHEMA arguments"
        usage
        exit 1
      fi
      validate_single_file "$2" "$3"
      ;;
    multiple)
      check_tools || exit 1
      if [[ $# -lt 3 ]]; then
        echo "Error: 'multiple' requires PATTERN and SCHEMA arguments"
        usage
        exit 1
      fi
      validate_multiple_files "$2" "$3" "${4:-file(s)}"
      ;;
    *)
      echo "Unknown command: $command"
      usage
      exit 1
      ;;
  esac
}

main "$@"
