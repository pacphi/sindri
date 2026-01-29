#!/usr/bin/env bash
# Version Consistency Validation Script for Sindri
# Usage: ./validate-versions.sh [--strict]
#
# Validates version consistency across:
#   - Git tags
#   - v2/cli/VERSION
#   - v3/Cargo.toml
#   - package.json
#   - v2/CHANGELOG.md
#   - v3/CHANGELOG.md
#
# Options:
#   --strict    Fail on warnings (use for pre-release validation)

set -euo pipefail

STRICT_MODE=false
if [[ "${1:-}" == "--strict" ]]; then
  STRICT_MODE=true
fi

ERRORS=0
WARNINGS=0

# Colors for output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

error() {
  echo -e "${RED}❌ ERROR: $1${NC}" >&2
  ((ERRORS++))
}

warning() {
  echo -e "${YELLOW}⚠️  WARNING: $1${NC}" >&2
  ((WARNINGS++))
}

success() {
  echo -e "${GREEN}✅ $1${NC}"
}

# Get latest git tags
get_latest_tag() {
  local prefix=$1
  git tag -l "${prefix}.*" --sort=-version:refname | head -1 || echo ""
}

# Extract version from file
get_version_from_file() {
  local file=$1
  local pattern=$2
  if [[ -f "$file" ]]; then
    grep -oP "$pattern" "$file" | head -1 || echo ""
  else
    echo ""
  fi
}

# Extract version from Cargo.toml
get_cargo_version() {
  local cargo_file="v3/Cargo.toml"
  if [[ -f "$cargo_file" ]]; then
    grep -oP '^version = "\K[^"]+' "$cargo_file" | head -1 || echo ""
  else
    echo ""
  fi
}

# Extract version from package.json
get_package_json_version() {
  local package_file="package.json"
  if [[ -f "$package_file" ]]; then
    grep -oP '"version": "\K[^"]+' "$package_file" | head -1 || echo ""
  else
    echo ""
  fi
}

# Check if version exists in changelog
check_changelog_version() {
  local changelog=$1
  local version=$2
  if [[ -f "$changelog" ]]; then
    if grep -q "## \[$version\]" "$changelog"; then
      return 0
    else
      return 1
    fi
  else
    return 1
  fi
}

echo "🔍 Validating version consistency..."
echo ""

# === v2 Version Validation ===
echo "Checking v2 versions..."

V2_TAG=$(get_latest_tag "v2")
V2_CLI_VERSION=""
if [[ -f "v2/cli/VERSION" ]]; then
  V2_CLI_VERSION=$(cat v2/cli/VERSION | tr -d '[:space:]')
fi

if [[ -n "$V2_TAG" ]]; then
  V2_TAG_VERSION="${V2_TAG#v}"
  echo "  Latest v2 tag: $V2_TAG"

  if [[ -n "$V2_CLI_VERSION" ]]; then
    echo "  v2/cli/VERSION: $V2_CLI_VERSION"
    if [[ "$V2_TAG_VERSION" == "$V2_CLI_VERSION" ]]; then
      success "v2 tag and v2/cli/VERSION match"
    else
      error "v2 tag ($V2_TAG_VERSION) != v2/cli/VERSION ($V2_CLI_VERSION)"
    fi
  else
    warning "v2/cli/VERSION file not found or empty"
  fi

  # Check v2 changelog
  if [[ -f "v2/CHANGELOG.md" ]]; then
    if check_changelog_version "v2/CHANGELOG.md" "$V2_TAG_VERSION"; then
      success "v2 CHANGELOG.md contains $V2_TAG_VERSION"
    else
      warning "v2 CHANGELOG.md missing entry for $V2_TAG_VERSION"
    fi
  fi
else
  echo "  No v2 tags found (skipping v2 validation)"
fi

echo ""

# === v3 Version Validation ===
echo "Checking v3 versions..."

V3_TAG=$(get_latest_tag "v3")
V3_CARGO_VERSION=$(get_cargo_version)

if [[ -n "$V3_TAG" ]]; then
  V3_TAG_VERSION="${V3_TAG#v}"
  echo "  Latest v3 tag: $V3_TAG"

  if [[ -n "$V3_CARGO_VERSION" ]]; then
    echo "  v3/Cargo.toml: $V3_CARGO_VERSION"
    if [[ "$V3_TAG_VERSION" == "$V3_CARGO_VERSION" ]]; then
      success "v3 tag and v3/Cargo.toml version match"
    else
      error "v3 tag ($V3_TAG_VERSION) != v3/Cargo.toml ($V3_CARGO_VERSION)"
    fi
  else
    warning "v3/Cargo.toml version not found"
  fi

  # Check v3 changelog
  if [[ -f "v3/CHANGELOG.md" ]]; then
    if check_changelog_version "v3/CHANGELOG.md" "$V3_TAG_VERSION"; then
      success "v3 CHANGELOG.md contains $V3_TAG_VERSION"
    else
      warning "v3 CHANGELOG.md missing entry for $V3_TAG_VERSION"
    fi
  fi
else
  echo "  No v3 tags found (skipping v3 validation)"
fi

echo ""

# === Cross-Version Commit Validation ===
echo "Checking for cross-version commits..."

# Only check if we have both v2 and v3 tags
if [[ -n "$V2_TAG" ]] && [[ -n "$V3_TAG" ]]; then
  # Find previous tags
  V2_PREV_TAG=$(git tag -l "v2.*" --sort=-version:refname | grep -A 1 "^$V2_TAG$" | tail -1 || echo "")
  V3_PREV_TAG=$(git tag -l "v3.*" --sort=-version:refname | grep -A 1 "^$V3_TAG$" | tail -1 || echo "")

  # Check for v3 changes in v2 release range
  if [[ -n "$V2_PREV_TAG" ]] && [[ "$V2_PREV_TAG" != "$V2_TAG" ]]; then
    V3_IN_V2=$(git log --oneline "$V2_PREV_TAG..$V2_TAG" -- v3/ 2>/dev/null | wc -l || echo "0")
    if [[ "$V3_IN_V2" -gt 0 ]]; then
      warning "Found $V3_IN_V2 v3/ changes in v2 release range ($V2_PREV_TAG..$V2_TAG)"
    else
      success "No v3/ changes in v2 release range"
    fi
  fi

  # Check for v2 changes in v3 release range
  if [[ -n "$V3_PREV_TAG" ]] && [[ "$V3_PREV_TAG" != "$V3_TAG" ]]; then
    V2_IN_V3=$(git log --oneline "$V3_PREV_TAG..$V3_TAG" -- v2/ 2>/dev/null | wc -l || echo "0")
    if [[ "$V2_IN_V3" -gt 0 ]]; then
      warning "Found $V2_IN_V3 v2/ changes in v3 release range ($V3_PREV_TAG..$V3_TAG)"
    else
      success "No v2/ changes in v3 release range"
    fi
  fi
fi

echo ""

# === package.json Validation (optional) ===
PACKAGE_VERSION=$(get_package_json_version)
if [[ -n "$PACKAGE_VERSION" ]]; then
  echo "Checking package.json version..."
  echo "  package.json: $PACKAGE_VERSION"

  # package.json doesn't need to match tags exactly (it's for tooling)
  # but we'll report it for awareness
  success "package.json version: $PACKAGE_VERSION (informational)"
  echo ""
fi

# === Summary ===
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Validation Summary:"
echo "  Errors:   $ERRORS"
echo "  Warnings: $WARNINGS"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [[ $ERRORS -gt 0 ]]; then
  echo ""
  error "Version validation failed with $ERRORS error(s)"
  exit 1
elif [[ $WARNINGS -gt 0 ]] && [[ "$STRICT_MODE" == "true" ]]; then
  echo ""
  error "Version validation failed in strict mode with $WARNINGS warning(s)"
  exit 1
elif [[ $WARNINGS -gt 0 ]]; then
  echo ""
  warning "Version validation passed with $WARNINGS warning(s)"
  exit 0
else
  echo ""
  success "All version checks passed!"
  exit 0
fi
