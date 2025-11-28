#!/usr/bin/env bash
# test/unit/yaml/test-yaml-lint.sh
# Run yamllint on all YAML files
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

echo "Running yamllint on all YAML files..."
echo ""

# Check if yamllint is available
if ! command -v yamllint &> /dev/null; then
  echo -e "${RED}ERROR: yamllint is required but not installed${NC}"
  echo "Install with: pip install yamllint"
  exit 1
fi

cd "$PROJECT_ROOT"

# Find all YAML files, excluding common directories
mapfile -t YAML_FILES < <(find . -type f \( -name "*.yaml" -o -name "*.yml" \) \
  ! -path "./node_modules/*" \
  ! -path "./.git/*" \
  ! -path "./vendor/*" \
  ! -path "./.venv/*" \
  2>/dev/null | sort)

if [[ ${#YAML_FILES[@]} -eq 0 ]]; then
  echo -e "${YELLOW}WARN: No YAML files found${NC}"
  exit 0
fi

echo "Found ${#YAML_FILES[@]} YAML file(s)"
echo ""

# Run yamllint with strict mode
if yamllint --strict "${YAML_FILES[@]}"; then
  echo ""
  echo -e "${GREEN}All YAML files pass linting${NC}"
else
  echo ""
  echo -e "${RED}YAML linting failed${NC}"
  exit 1
fi
