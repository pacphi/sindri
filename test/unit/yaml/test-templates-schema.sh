#!/usr/bin/env bash
# test/unit/yaml/test-templates-schema.sh
# Validate project-templates.yaml against its schema
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

SCHEMA="$PROJECT_ROOT/docker/lib/schemas/project-templates.schema.json"
TEMPLATES_FILE="$PROJECT_ROOT/docker/lib/project-templates.yaml"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

echo "Validating project-templates.yaml against schema..."
echo "Schema: $SCHEMA"
echo "File: $TEMPLATES_FILE"
echo ""

# Check if required tools are available
if ! command -v yq &> /dev/null; then
  echo -e "${RED}ERROR: yq is required but not installed${NC}"
  exit 1
fi

# Basic YAML syntax validation
if ! yq '.' "$TEMPLATES_FILE" > /dev/null 2>&1; then
  echo -e "${RED}FAIL: project-templates.yaml - Invalid YAML syntax${NC}"
  exit 1
fi

# Validate required fields
if ! yq -e '.version' "$TEMPLATES_FILE" > /dev/null 2>&1; then
  echo -e "${RED}FAIL: project-templates.yaml - Missing 'version' field${NC}"
  exit 1
fi

if ! yq -e '.templates' "$TEMPLATES_FILE" > /dev/null 2>&1; then
  echo -e "${RED}FAIL: project-templates.yaml - Missing 'templates' field${NC}"
  exit 1
fi

# Validate each template has required fields
FAILURES=0
for template in $(yq '.templates | keys | .[]' "$TEMPLATES_FILE"); do
  if ! yq -e ".templates.$template.description" "$TEMPLATES_FILE" > /dev/null 2>&1; then
    echo -e "${RED}FAIL: Template '$template' missing 'description'${NC}"
    ((FAILURES++))
  fi

  if ! yq -e ".templates.$template.extensions" "$TEMPLATES_FILE" > /dev/null 2>&1; then
    echo -e "${RED}FAIL: Template '$template' missing 'extensions'${NC}"
    ((FAILURES++))
  fi
done

if [[ $FAILURES -gt 0 ]]; then
  echo -e "${RED}Validation failed: $FAILURES error(s)${NC}"
  exit 1
fi

# Schema validation with ajv if available
if command -v ajv &> /dev/null; then
  if ! yq -o=json "$TEMPLATES_FILE" | ajv validate -s "$SCHEMA" -d /dev/stdin 2>/dev/null; then
    echo -e "${RED}FAIL: project-templates.yaml - Schema validation failed${NC}"
    exit 1
  fi
fi

echo -e "${GREEN}PASS: project-templates.yaml is valid${NC}"
