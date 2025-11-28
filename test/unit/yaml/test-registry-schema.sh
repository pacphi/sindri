#!/usr/bin/env bash
# test/unit/yaml/test-registry-schema.sh
# Validate registry.yaml against its schema
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

SCHEMA="$PROJECT_ROOT/docker/lib/schemas/registry.schema.json"
REGISTRY_FILE="$PROJECT_ROOT/docker/lib/registry.yaml"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

echo "Validating registry.yaml against schema..."
echo "Schema: $SCHEMA"
echo "File: $REGISTRY_FILE"
echo ""

# Check if required tools are available
if ! command -v yq &> /dev/null; then
  echo -e "${RED}ERROR: yq is required but not installed${NC}"
  exit 1
fi

# Basic YAML syntax validation
if ! yq '.' "$REGISTRY_FILE" > /dev/null 2>&1; then
  echo -e "${RED}FAIL: registry.yaml - Invalid YAML syntax${NC}"
  exit 1
fi

# Validate required fields
if ! yq -e '.version' "$REGISTRY_FILE" > /dev/null 2>&1; then
  echo -e "${RED}FAIL: registry.yaml - Missing 'version' field${NC}"
  exit 1
fi

if ! yq -e '.extensions' "$REGISTRY_FILE" > /dev/null 2>&1; then
  echo -e "${RED}FAIL: registry.yaml - Missing 'extensions' field${NC}"
  exit 1
fi

# Validate each extension has required fields
FAILURES=0
VALID_CATEGORIES="base language dev-tools infrastructure ai database monitoring mobile desktop utilities"

for ext in $(yq '.extensions | keys | .[]' "$REGISTRY_FILE"); do
  if ! yq -e ".extensions.$ext.category" "$REGISTRY_FILE" > /dev/null 2>&1; then
    echo -e "${RED}FAIL: Extension '$ext' missing 'category'${NC}"
    ((FAILURES++))
    continue
  fi

  category=$(yq ".extensions.$ext.category" "$REGISTRY_FILE")
  if [[ ! " $VALID_CATEGORIES " =~ \ $category\  ]]; then
    echo -e "${RED}FAIL: Extension '$ext' has invalid category '$category'${NC}"
    ((FAILURES++))
  fi

  if ! yq -e ".extensions.$ext.description" "$REGISTRY_FILE" > /dev/null 2>&1; then
    echo -e "${RED}FAIL: Extension '$ext' missing 'description'${NC}"
    ((FAILURES++))
  fi
done

if [[ $FAILURES -gt 0 ]]; then
  echo -e "${RED}Validation failed: $FAILURES error(s)${NC}"
  exit 1
fi

# Schema validation with ajv if available
if command -v ajv &> /dev/null; then
  if ! yq -o=json "$REGISTRY_FILE" | ajv validate -s "$SCHEMA" -d /dev/stdin 2>/dev/null; then
    echo -e "${RED}FAIL: registry.yaml - Schema validation failed${NC}"
    exit 1
  fi
fi

echo -e "${GREEN}PASS: registry.yaml is valid${NC}"
