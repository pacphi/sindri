#!/usr/bin/env bash
# test/unit/yaml/test-categories-schema.sh
# Validate categories.yaml against its schema
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

SCHEMA="$PROJECT_ROOT/docker/lib/schemas/categories.schema.json"
CATEGORIES_FILE="$PROJECT_ROOT/docker/lib/categories.yaml"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

echo "Validating categories.yaml against schema..."
echo "Schema: $SCHEMA"
echo "File: $CATEGORIES_FILE"
echo ""

# Check if required tools are available
if ! command -v yq &> /dev/null; then
  echo -e "${RED}ERROR: yq is required but not installed${NC}"
  exit 1
fi

# Basic YAML syntax validation
if ! yq '.' "$CATEGORIES_FILE" > /dev/null 2>&1; then
  echo -e "${RED}FAIL: categories.yaml - Invalid YAML syntax${NC}"
  exit 1
fi

# Validate required fields
if ! yq -e '.version' "$CATEGORIES_FILE" > /dev/null 2>&1; then
  echo -e "${RED}FAIL: categories.yaml - Missing 'version' field${NC}"
  exit 1
fi

if ! yq -e '.categories' "$CATEGORIES_FILE" > /dev/null 2>&1; then
  echo -e "${RED}FAIL: categories.yaml - Missing 'categories' field${NC}"
  exit 1
fi

# Validate each category has required fields
FAILURES=0
for cat in $(yq '.categories | keys | .[]' "$CATEGORIES_FILE"); do
  if ! yq -e ".categories.$cat.name" "$CATEGORIES_FILE" > /dev/null 2>&1; then
    echo -e "${RED}FAIL: Category '$cat' missing 'name'${NC}"
    ((FAILURES++))
  fi

  if ! yq -e ".categories.$cat.description" "$CATEGORIES_FILE" > /dev/null 2>&1; then
    echo -e "${RED}FAIL: Category '$cat' missing 'description'${NC}"
    ((FAILURES++))
  fi

  if ! yq -e ".categories.$cat.priority" "$CATEGORIES_FILE" > /dev/null 2>&1; then
    echo -e "${RED}FAIL: Category '$cat' missing 'priority'${NC}"
    ((FAILURES++))
  fi

  # Validate priority is a positive integer
  priority=$(yq ".categories.$cat.priority" "$CATEGORIES_FILE")
  if ! [[ "$priority" =~ ^[1-9][0-9]*$ ]]; then
    echo -e "${RED}FAIL: Category '$cat' has invalid priority '$priority' (must be positive integer)${NC}"
    ((FAILURES++))
  fi
done

if [[ $FAILURES -gt 0 ]]; then
  echo -e "${RED}Validation failed: $FAILURES error(s)${NC}"
  exit 1
fi

# Schema validation with ajv if available
if command -v ajv &> /dev/null; then
  if ! yq -o=json "$CATEGORIES_FILE" | ajv validate -s "$SCHEMA" -d /dev/stdin 2>/dev/null; then
    echo -e "${RED}FAIL: categories.yaml - Schema validation failed${NC}"
    exit 1
  fi
fi

echo -e "${GREEN}PASS: categories.yaml is valid${NC}"
