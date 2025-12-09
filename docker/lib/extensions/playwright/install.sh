#!/bin/bash
set -euo pipefail

# playwright install script - Simplified for YAML-driven architecture
# Installs Playwright browser automation framework in workspace

# Source common utilities
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Installing Playwright..."

# Use WORKSPACE environment variable with fallback
WORKSPACE="${WORKSPACE:-${HOME}/workspace}"
cd "$WORKSPACE" || exit 1

# Check if already installed
if [[ -f "node_modules/.bin/playwright" ]]; then
  if pw_version=$(npx playwright --version 2>/dev/null); then
    print_warning "Playwright already installed: $pw_version"
    exit 0
  fi
fi

# Initialize package.json if needed
if [[ ! -f "package.json" ]]; then
  print_status "Initializing Node.js project..."
  npm init -y || exit 1
fi

# Set up ES modules
npm pkg set type="module"

# Set memory limit
export NODE_OPTIONS="${NODE_OPTIONS:---max-old-space-size=6144}"

# Install system dependencies
print_status "Installing Playwright system dependencies..."
npx playwright install-deps || print_warning "Some system dependencies may be missing"

# Check CI mode
if [[ "${CI:-}" == "true" ]] || [[ "${GITHUB_ACTIONS:-}" == "true" ]]; then
  print_status "CI mode: Installing Playwright (browsers skipped)..."
  export PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1
  npm install -D playwright @playwright/test --prefer-offline --no-audit --no-fund || exit 1
  print_success "Playwright packages installed (browsers skipped for CI)"
else
  # Full installation with browsers
  print_status "Installing Playwright packages..."
  npm install -D playwright @playwright/test || exit 1
  print_success "Playwright packages installed"

  print_status "Installing Chromium browser..."
  npx playwright install chromium && print_success "Chromium installed"
fi

# Install TypeScript
print_status "Installing TypeScript..."
npm install -D typescript @types/node && print_success "TypeScript installed"

# Create tests directory
mkdir -p tests

print_success "Playwright installation complete"
