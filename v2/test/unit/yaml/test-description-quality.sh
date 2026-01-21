#!/usr/bin/env bash
# test/unit/yaml/test-description-quality.sh
# Ensure descriptions are meaningful (not too short, not placeholder)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

EXTENSIONS_DIR="$PROJECT_ROOT/docker/lib/extensions"
WARNINGS=0

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

echo "Checking description quality..."
echo ""

# Check if required tools are available
if ! command -v yq &> /dev/null; then
  echo -e "${RED}ERROR: yq is required but not installed${NC}"
  exit 1
fi

for ext in "$EXTENSIONS_DIR"/*/extension.yaml; do
  ext_name=$(basename "$(dirname "$ext")")
  desc=$(yq '.metadata.description // ""' "$ext" 2>/dev/null || echo "")

  # Check for empty description
  if [[ -z "$desc" ]]; then
    echo -e "${YELLOW}WARN: $ext_name has empty description${NC}"
    ((WARNINGS++)) || true
    continue
  fi

  # Check minimum length
  if [[ ${#desc} -lt 10 ]]; then
    echo -e "${YELLOW}WARN: $ext_name has short description: '$desc'${NC}"
    ((WARNINGS++)) || true
  fi

  # Check for placeholder text (case-insensitive)
  desc_lower=$(echo "$desc" | tr '[:upper:]' '[:lower:]')
  if [[ "$desc_lower" =~ (todo|fixme|placeholder|description\ here|add\ description) ]]; then
    echo -e "${YELLOW}WARN: $ext_name may have placeholder description: '$desc'${NC}"
    ((WARNINGS++)) || true
  fi
done

# Also check profiles
echo ""
echo "Checking profile descriptions..."

PROFILES_FILE="$PROJECT_ROOT/docker/lib/profiles.yaml"
for profile in $(yq '.profiles | keys | .[]' "$PROFILES_FILE" 2>/dev/null); do
  desc=$(yq ".profiles.$profile.description // \"\"" "$PROFILES_FILE" 2>/dev/null || echo "")

  if [[ -z "$desc" ]]; then
    echo -e "${YELLOW}WARN: Profile '$profile' has empty description${NC}"
    ((WARNINGS++)) || true
  elif [[ ${#desc} -lt 10 ]]; then
    echo -e "${YELLOW}WARN: Profile '$profile' has short description: '$desc'${NC}"
    ((WARNINGS++)) || true
  fi
done

echo ""
echo "================================"

if [[ $WARNINGS -gt 0 ]]; then
  echo -e "${YELLOW}Found $WARNINGS description quality warning(s)${NC}"
fi

echo -e "${GREEN}Description quality check complete${NC}"
