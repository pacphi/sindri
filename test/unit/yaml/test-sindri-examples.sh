#!/usr/bin/env bash
# test/unit/yaml/test-sindri-examples.sh
# Validate all sindri.yaml example files against the schema
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

SCHEMA="$PROJECT_ROOT/docker/lib/schemas/sindri.schema.json"
EXAMPLES_DIR="$PROJECT_ROOT/examples"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

echo "Validating sindri.yaml examples against schema..."
echo "Schema: $SCHEMA"
echo "Examples dir: $EXAMPLES_DIR"
echo ""

# Check if required tools are available
if ! command -v yq &> /dev/null; then
  echo -e "${RED}ERROR: yq is required but not installed${NC}"
  exit 1
fi

# Find all sindri.yaml files and store in a temp file
TMPFILE=$(mktemp)
trap 'rm -f "$TMPFILE"' EXIT

find "$EXAMPLES_DIR" -name "*.sindri.yaml" -type f 2>/dev/null > "$TMPFILE" || true

EXAMPLE_COUNT=$(wc -l < "$TMPFILE" | tr -d ' ')

if [[ "$EXAMPLE_COUNT" -eq 0 ]]; then
  echo -e "${YELLOW}WARN: No sindri.yaml examples found in $EXAMPLES_DIR${NC}"
  exit 0
fi

echo "Found $EXAMPLE_COUNT example file(s)"
echo ""

FAILURES=0
PASSED=0

while IFS= read -r example; do
  [[ -z "$example" ]] && continue

  rel_path="${example#"$PROJECT_ROOT"/}"

  # Basic YAML syntax validation
  if ! yq '.' "$example" > /dev/null 2>&1; then
    echo -e "${RED}FAIL: $rel_path - Invalid YAML syntax${NC}"
    ((FAILURES++))
    continue
  fi

  # Validate required fields
  VALID=true

  if ! yq -e '.version' "$example" > /dev/null 2>&1; then
    echo -e "${RED}FAIL: $rel_path - Missing 'version'${NC}"
    VALID=false
  fi

  if ! yq -e '.name' "$example" > /dev/null 2>&1; then
    echo -e "${RED}FAIL: $rel_path - Missing 'name'${NC}"
    VALID=false
  fi

  if ! yq -e '.deployment.provider' "$example" > /dev/null 2>&1; then
    echo -e "${RED}FAIL: $rel_path - Missing 'deployment.provider'${NC}"
    VALID=false
  fi

  if ! yq -e '.extensions' "$example" > /dev/null 2>&1; then
    echo -e "${RED}FAIL: $rel_path - Missing 'extensions'${NC}"
    VALID=false
  fi

  if [[ "$VALID" == "false" ]]; then
    ((FAILURES++))
    continue
  fi

  # Schema validation with ajv if available
  if command -v ajv &> /dev/null; then
    if ! yq -o=json "$example" | ajv validate -s "$SCHEMA" -d /dev/stdin 2>/dev/null; then
      echo -e "${RED}FAIL: $rel_path - Schema validation failed${NC}"
      ((FAILURES++))
    else
      echo -e "${GREEN}PASS: $rel_path${NC}"
      ((PASSED++))
    fi
  else
    echo -e "${GREEN}PASS: $rel_path (syntax only)${NC}"
    ((PASSED++))
  fi
done < "$TMPFILE"

echo ""
echo "================================"
echo "Results: $PASSED passed, $FAILURES failed"
echo "================================"

if [[ $FAILURES -gt 0 ]]; then
  echo -e "${RED}Validation failed: $FAILURES example(s)${NC}"
  exit 1
fi

echo -e "${GREEN}All sindri.yaml examples valid${NC}"
