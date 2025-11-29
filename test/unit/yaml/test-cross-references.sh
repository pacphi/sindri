#!/usr/bin/env bash
# test/unit/yaml/test-cross-references.sh
# Validates that references between YAML files are consistent
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

FAILURES=0

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

echo "Validating cross-file references..."
echo ""

# Check if required tools are available
if ! command -v yq &> /dev/null; then
  echo -e "${RED}ERROR: yq is required but not installed${NC}"
  exit 1
fi

cd "$PROJECT_ROOT"

# 1. All extensions in registry.yaml must have extension.yaml files
echo "Checking registry -> extensions..."
for ext in $(yq '.extensions | keys | .[]' docker/lib/registry.yaml 2>/dev/null); do
  if [[ ! -f "docker/lib/extensions/$ext/extension.yaml" ]]; then
    echo -e "${RED}FAIL: registry.yaml references '$ext' but no extension.yaml exists${NC}"
    ((FAILURES++))
  fi
done
echo -e "${GREEN}  Done${NC}"

# 2. All extensions in profiles.yaml must exist in registry.yaml
echo "Checking profiles -> registry..."
for profile in $(yq '.profiles | keys | .[]' docker/lib/profiles.yaml 2>/dev/null); do
  for ext in $(yq ".profiles.$profile.extensions[]" docker/lib/profiles.yaml 2>/dev/null); do
    if ! yq -e ".extensions.$ext" docker/lib/registry.yaml > /dev/null 2>&1; then
      echo -e "${RED}FAIL: profile '$profile' references extension '$ext' not in registry${NC}"
      ((FAILURES++))
    fi
  done
done
echo -e "${GREEN}  Done${NC}"

# 3. All categories in registry.yaml must exist in categories.yaml
echo "Checking registry -> categories..."
for ext in $(yq '.extensions | keys | .[]' docker/lib/registry.yaml 2>/dev/null); do
  category=$(yq ".extensions.$ext.category" docker/lib/registry.yaml 2>/dev/null)
  if [[ -n "$category" ]] && ! yq -e ".categories.$category" docker/lib/categories.yaml > /dev/null 2>&1; then
    echo -e "${RED}FAIL: extension '$ext' has category '$category' not in categories.yaml${NC}"
    ((FAILURES++))
  fi
done
echo -e "${GREEN}  Done${NC}"

# 4. All extension dependencies must exist
echo "Checking extension dependencies..."
for ext_dir in docker/lib/extensions/*/; do
  ext_name=$(basename "$ext_dir")
  ext_file="$ext_dir/extension.yaml"

  if [[ -f "$ext_file" ]]; then
    # Get dependencies array if it exists
    deps=$(yq '.metadata.dependencies // []' "$ext_file" 2>/dev/null | yq '.[]' 2>/dev/null || true)
    for dep in $deps; do
      if [[ -n "$dep" ]] && [[ ! -d "docker/lib/extensions/$dep" ]]; then
        echo -e "${RED}FAIL: $ext_name depends on '$dep' which doesn't exist${NC}"
        ((FAILURES++))
      fi
    done
  fi
done
echo -e "${GREEN}  Done${NC}"

# 5. Sindri.yaml examples reference valid profiles
echo "Checking examples -> profiles..."
for example in $(find examples -name "*.sindri.yaml" 2>/dev/null || true); do
  profile=$(yq '.extensions.profile // ""' "$example" 2>/dev/null)
  if [[ -n "$profile" ]] && ! yq -e ".profiles.$profile" docker/lib/profiles.yaml > /dev/null 2>&1; then
    echo -e "${RED}FAIL: $example references profile '$profile' not in profiles.yaml${NC}"
    ((FAILURES++))
  fi
done
echo -e "${GREEN}  Done${NC}"

# 6. Template extensions exist in registry
echo "Checking templates -> registry..."
for template in $(yq '.templates | keys | .[]' docker/lib/project-templates.yaml 2>/dev/null); do
  for ext in $(yq ".templates.$template.extensions[]" docker/lib/project-templates.yaml 2>/dev/null); do
    if ! yq -e ".extensions.$ext" docker/lib/registry.yaml > /dev/null 2>&1; then
      echo -e "${YELLOW}WARN: template '$template' references extension '$ext' not in registry${NC}"
    fi
  done
done
echo -e "${GREEN}  Done${NC}"

echo ""
echo "================================"

if [[ $FAILURES -gt 0 ]]; then
  echo -e "${RED}Cross-reference validation failed: $FAILURES error(s)${NC}"
  exit 1
fi

echo -e "${GREEN}All cross-references valid${NC}"
