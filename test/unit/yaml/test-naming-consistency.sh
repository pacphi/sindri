#!/usr/bin/env bash
# test/unit/yaml/test-naming-consistency.sh
# Check extension naming and category consistency
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

EXTENSIONS_DIR="$PROJECT_ROOT/docker/lib/extensions"
REGISTRY_FILE="$PROJECT_ROOT/docker/lib/registry.yaml"
FAILURES=0

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

echo "Checking naming and category consistency..."
echo ""

# Check if required tools are available
if ! command -v yq &> /dev/null; then
  echo -e "${RED}ERROR: yq is required but not installed${NC}"
  exit 1
fi

# 1. Extension directory name must match metadata.name
echo "Checking extension naming consistency..."
for ext_dir in "$EXTENSIONS_DIR"/*/; do
  ext_file="$ext_dir/extension.yaml"

  if [[ -f "$ext_file" ]]; then
    dir_name=$(basename "$ext_dir")
    yaml_name=$(yq '.metadata.name // ""' "$ext_file" 2>/dev/null || echo "")

    if [[ -n "$yaml_name" ]] && [[ "$dir_name" != "$yaml_name" ]]; then
      echo -e "${RED}FAIL: Directory '$dir_name' doesn't match metadata.name '$yaml_name'${NC}"
      ((FAILURES++))
    fi
  fi
done
echo -e "${GREEN}  Done${NC}"

# 2. Extension category must match registry entry
echo "Checking category consistency..."
for ext_dir in "$EXTENSIONS_DIR"/*/; do
  ext_file="$ext_dir/extension.yaml"

  if [[ -f "$ext_file" ]]; then
    ext_name=$(basename "$ext_dir")
    ext_category=$(yq '.metadata.category // ""' "$ext_file" 2>/dev/null || echo "")
    reg_category=$(yq ".extensions.$ext_name.category // \"\"" "$REGISTRY_FILE" 2>/dev/null || echo "")

    if [[ -n "$ext_category" ]] && [[ -n "$reg_category" ]] && \
       [[ "$ext_category" != "$reg_category" ]]; then
      echo -e "${RED}FAIL: $ext_name category mismatch: extension.yaml='$ext_category', registry.yaml='$reg_category'${NC}"
      ((FAILURES++))
    fi
  fi
done
echo -e "${GREEN}  Done${NC}"

# 3. Check that extension names follow conventions (lowercase, hyphens)
echo "Checking extension naming conventions..."
for ext_dir in "$EXTENSIONS_DIR"/*/; do
  ext_name=$(basename "$ext_dir")

  # Should be lowercase with hyphens, not underscores
  if [[ "$ext_name" =~ [A-Z] ]]; then
    echo -e "${YELLOW}WARN: Extension '$ext_name' contains uppercase letters${NC}"
  fi

  if [[ "$ext_name" =~ _ ]]; then
    echo -e "${YELLOW}WARN: Extension '$ext_name' contains underscores (use hyphens instead)${NC}"
  fi

  # Should not start with a number
  if [[ "$ext_name" =~ ^[0-9] ]]; then
    echo -e "${YELLOW}WARN: Extension '$ext_name' starts with a number${NC}"
  fi
done
echo -e "${GREEN}  Done${NC}"

echo ""
echo "================================"

if [[ $FAILURES -gt 0 ]]; then
  echo -e "${RED}Naming consistency check failed: $FAILURES error(s)${NC}"
  exit 1
fi

echo -e "${GREEN}All naming and category checks passed${NC}"
