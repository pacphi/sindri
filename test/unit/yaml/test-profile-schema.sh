#!/usr/bin/env bash
# test/unit/yaml/test-profile-schema.sh
# Validate profiles.yaml against its schema
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

SCHEMA="$PROJECT_ROOT/docker/lib/schemas/profiles.schema.json"
PROFILES_FILE="$PROJECT_ROOT/docker/lib/profiles.yaml"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

echo "Validating profiles.yaml against schema..."
echo "Schema: $SCHEMA"
echo "File: $PROFILES_FILE"
echo ""

# Check if required tools are available
if ! command -v yq &> /dev/null; then
  echo -e "${RED}ERROR: yq is required but not installed${NC}"
  exit 1
fi

# Basic YAML syntax validation
if ! yq '.' "$PROFILES_FILE" > /dev/null 2>&1; then
  echo -e "${RED}FAIL: profiles.yaml - Invalid YAML syntax${NC}"
  exit 1
fi

# Validate required fields
if ! yq -e '.version' "$PROFILES_FILE" > /dev/null 2>&1; then
  echo -e "${RED}FAIL: profiles.yaml - Missing 'version' field${NC}"
  exit 1
fi

if ! yq -e '.profiles' "$PROFILES_FILE" > /dev/null 2>&1; then
  echo -e "${RED}FAIL: profiles.yaml - Missing 'profiles' field${NC}"
  exit 1
fi

# Validate each profile has required fields
FAILURES=0
for profile in $(yq '.profiles | keys | .[]' "$PROFILES_FILE"); do
  if ! yq -e ".profiles.$profile.description" "$PROFILES_FILE" > /dev/null 2>&1; then
    echo -e "${RED}FAIL: Profile '$profile' missing 'description'${NC}"
    ((FAILURES++))
  fi

  if ! yq -e ".profiles.$profile.extensions" "$PROFILES_FILE" > /dev/null 2>&1; then
    echo -e "${RED}FAIL: Profile '$profile' missing 'extensions'${NC}"
    ((FAILURES++))
  fi

  # Check extensions is a non-empty array
  ext_count=$(yq ".profiles.$profile.extensions | length" "$PROFILES_FILE")
  if [[ "$ext_count" -eq 0 ]]; then
    echo -e "${RED}FAIL: Profile '$profile' has empty extensions list${NC}"
    ((FAILURES++))
  fi
done

if [[ $FAILURES -gt 0 ]]; then
  echo -e "${RED}Validation failed: $FAILURES error(s)${NC}"
  exit 1
fi

# Schema validation with ajv if available
if command -v ajv &> /dev/null; then
  if ! yq -o=json "$PROFILES_FILE" | ajv validate -s "$SCHEMA" -d /dev/stdin 2>/dev/null; then
    echo -e "${RED}FAIL: profiles.yaml - Schema validation failed${NC}"
    exit 1
  fi
fi

echo -e "${GREEN}PASS: profiles.yaml is valid${NC}"
