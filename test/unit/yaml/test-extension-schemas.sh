#!/usr/bin/env bash
# test/unit/yaml/test-extension-schemas.sh
# Validate all extension.yaml files against the schema
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

SCHEMA="$PROJECT_ROOT/docker/lib/schemas/extension.schema.json"
EXTENSIONS_DIR="$PROJECT_ROOT/docker/lib/extensions"
FAILURES=0
PASSED=0

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo "Validating extension.yaml files against schema..."
echo "Schema: $SCHEMA"
echo "Extensions dir: $EXTENSIONS_DIR"
echo ""

# Check if required tools are available
if ! command -v yq &> /dev/null; then
  echo -e "${RED}ERROR: yq is required but not installed${NC}"
  echo "Install with: brew install yq (macOS) or pip install yq (Linux)"
  exit 1
fi

if ! command -v ajv &> /dev/null; then
  echo -e "${YELLOW}WARN: ajv-cli not found, using basic YAML validation only${NC}"
  AJV_AVAILABLE=false
else
  AJV_AVAILABLE=true
fi

for ext_dir in "$EXTENSIONS_DIR"/*/; do
  ext_name=$(basename "$ext_dir")
  ext_file="$ext_dir/extension.yaml"

  if [[ ! -f "$ext_file" ]]; then
    echo -e "${YELLOW}WARN: $ext_name has no extension.yaml${NC}"
    continue
  fi

  # Basic YAML syntax validation
  if ! yq '.' "$ext_file" > /dev/null 2>&1; then
    echo -e "${RED}FAIL: $ext_name/extension.yaml - Invalid YAML syntax${NC}"
    ((FAILURES++))
    continue
  fi

  # Schema validation if ajv is available
  if [[ "$AJV_AVAILABLE" == "true" ]]; then
    if ! yq -o=json "$ext_file" | ajv validate -s "$SCHEMA" -d /dev/stdin 2>/dev/null; then
      echo -e "${RED}FAIL: $ext_name/extension.yaml - Schema validation failed${NC}"
      ((FAILURES++))
    else
      echo -e "${GREEN}PASS: $ext_name/extension.yaml${NC}"
      ((PASSED++))
    fi
  else
    # Just validate required fields exist
    if yq -e '.metadata.name' "$ext_file" > /dev/null 2>&1; then
      echo -e "${GREEN}PASS: $ext_name/extension.yaml (syntax only)${NC}"
      ((PASSED++))
    else
      echo -e "${RED}FAIL: $ext_name/extension.yaml - Missing metadata.name${NC}"
      ((FAILURES++))
    fi
  fi
done

echo ""
echo "================================"
echo "Results: $PASSED passed, $FAILURES failed"
echo "================================"

if [[ $FAILURES -gt 0 ]]; then
  echo -e "${RED}Schema validation failed: $FAILURES extension(s)${NC}"
  exit 1
fi

echo -e "${GREEN}All extension.yaml files valid${NC}"
