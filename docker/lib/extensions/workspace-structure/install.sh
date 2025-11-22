#!/bin/bash
set -euo pipefail

# workspace-structure install script - Simplified for YAML-driven architecture
# Creates workspace directory structure

# Source common utilities
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Creating workspace structure..."

mkdir -p /workspace/{projects,config,scripts,bin,.local,.config,.system/{manifest,installed,logs}}
chown -R developer:developer /workspace 2>/dev/null || true

print_success "Workspace structure created"
