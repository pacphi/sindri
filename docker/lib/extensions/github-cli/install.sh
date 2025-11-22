#!/bin/bash
set -euo pipefail

# github-cli install script - Simplified for YAML-driven architecture
# GitHub CLI is pre-installed in the Docker image
# This script only handles authentication configuration

# Source common utilities
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Configuring GitHub CLI..."

# GitHub CLI is pre-installed via Docker build
version=$(gh version 2>/dev/null | head -n1 || echo "unknown")
print_success "GitHub CLI already installed: $version"

# Authentication will be handled by entrypoint.sh using GITHUB_TOKEN env var
print_status "GitHub CLI authentication will be configured at container startup"
print_status "Set GITHUB_TOKEN environment variable or run: gh auth login"

print_success "GitHub CLI configuration complete"
