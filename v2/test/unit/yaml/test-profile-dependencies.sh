#!/usr/bin/env bash
# test/unit/yaml/test-profile-dependencies.sh
# Ensure profile extensions are in dependency order
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

PROFILES_FILE="$PROJECT_ROOT/v2/docker/lib/profiles.yaml"
EXTENSIONS_DIR="$PROJECT_ROOT/v2/docker/lib/extensions"
WARNINGS=0

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

echo "Checking profile dependency ordering..."
echo ""

# Check if required tools are available
if ! command -v yq &> /dev/null; then
  echo -e "${RED}ERROR: yq is required but not installed${NC}"
  exit 1
fi

for profile in $(yq '.profiles | keys | .[]' "$PROFILES_FILE" 2>/dev/null); do
  echo "Checking profile: $profile"

  # Get list of extensions in order (bash 3.2 compatible - no mapfile)
  extensions=()
  while IFS= read -r line; do
    [[ -n "$line" ]] && extensions+=("$line")
  done < <(yq ".profiles.$profile.extensions[]" "$PROFILES_FILE" 2>/dev/null)

  # For each extension, check that its dependencies come before it
  for i in "${!extensions[@]}"; do
    ext="${extensions[$i]}"
    ext_file="$EXTENSIONS_DIR/$ext/extension.yaml"

    if [[ ! -f "$ext_file" ]]; then
      continue
    fi

    # Get dependencies for this extension (bash 3.2 compatible)
    deps=()
    while IFS= read -r line; do
      [[ -n "$line" ]] && deps+=("$line")
    done < <(yq '.metadata.dependencies // [] | .[]' "$ext_file" 2>/dev/null || true)

    # Handle empty deps array safely with set -u
    for dep in ${deps[@]+"${deps[@]}"}; do
      if [[ -z "$dep" ]]; then
        continue
      fi

      # Find position of dependency in the list
      dep_position=-1
      for j in "${!extensions[@]}"; do
        if [[ "${extensions[$j]}" == "$dep" ]]; then
          dep_position=$j
          break
        fi
      done

      # If dependency is in the list but comes after the extension
      if [[ $dep_position -gt $i ]]; then
        echo -e "${YELLOW}WARN: Profile '$profile': $ext depends on $dep which comes later (position $i vs $dep_position)${NC}"
        ((WARNINGS++)) || true
      fi
    done
  done
done

echo ""
echo "================================"

if [[ $WARNINGS -gt 0 ]]; then
  echo -e "${YELLOW}Found $WARNINGS dependency ordering warning(s)${NC}"
  echo "Consider reordering extensions in profiles.yaml"
fi

echo -e "${GREEN}Profile dependency check complete${NC}"
