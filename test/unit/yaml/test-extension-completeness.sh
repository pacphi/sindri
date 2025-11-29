#!/usr/bin/env bash
# test/unit/yaml/test-extension-completeness.sh
# Ensure all extensions have required components
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

EXTENSIONS_DIR="$PROJECT_ROOT/docker/lib/extensions"
FAILURES=0

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

echo "Checking extension completeness..."
echo ""

# Check if required tools are available
if ! command -v yq &> /dev/null; then
  echo -e "${RED}ERROR: yq is required but not installed${NC}"
  exit 1
fi

for ext_dir in "$EXTENSIONS_DIR"/*/; do
  ext_name=$(basename "$ext_dir")
  ext_file="$ext_dir/extension.yaml"

  # Must have extension.yaml
  if [[ ! -f "$ext_file" ]]; then
    echo -e "${RED}FAIL: $ext_name missing extension.yaml${NC}"
    ((FAILURES++))
    continue
  fi

  # Check install method
  method=$(yq '.install.method // ""' "$ext_file" 2>/dev/null || echo "")

  if [[ "$method" == "mise" ]]; then
    # If install.method is 'mise', check for mise.toml or inline config
    mise_config=$(yq '.install.mise.configFile // ""' "$ext_file" 2>/dev/null || echo "")
    mise_inline=$(yq '.install.mise.tools // ""' "$ext_file" 2>/dev/null || echo "")

    if [[ -n "$mise_config" ]] && [[ ! -f "$ext_dir/$mise_config" ]]; then
      echo -e "${RED}FAIL: $ext_name uses mise but missing $mise_config${NC}"
      ((FAILURES++))
    elif [[ -z "$mise_config" ]] && [[ -z "$mise_inline" ]]; then
      echo -e "${YELLOW}WARN: $ext_name uses mise but no config found${NC}"
    fi
  fi

  if [[ "$method" == "script" ]]; then
    # If install.method is 'script', must have the script file
    script_path=$(yq '.install.script.path // ""' "$ext_file" 2>/dev/null || echo "")
    if [[ -n "$script_path" ]] && [[ ! -f "$ext_dir/$script_path" ]]; then
      echo -e "${RED}FAIL: $ext_name references script '$script_path' but file missing${NC}"
      ((FAILURES++))
    fi
  fi

  # Check metadata.name matches directory name
  yaml_name=$(yq '.metadata.name // ""' "$ext_file" 2>/dev/null || echo "")
  if [[ -n "$yaml_name" ]] && [[ "$ext_name" != "$yaml_name" ]]; then
    echo -e "${YELLOW}WARN: $ext_name directory doesn't match metadata.name '$yaml_name'${NC}"
  fi
done

echo ""
echo "================================"

if [[ $FAILURES -gt 0 ]]; then
  echo -e "${RED}Extension completeness check failed: $FAILURES error(s)${NC}"
  exit 1
fi

echo -e "${GREEN}All extensions have required components${NC}"
