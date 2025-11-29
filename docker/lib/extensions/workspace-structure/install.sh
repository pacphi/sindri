#!/bin/bash
set -euo pipefail

# workspace-structure install script - Simplified for YAML-driven architecture
# Creates workspace directory structure

# Source common utilities
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

# Use WORKSPACE from environment or derive from HOME
WORKSPACE="${WORKSPACE:-${HOME}/workspace}"

print_status "Creating workspace structure..."

mkdir -p "${WORKSPACE}"/{projects,config,scripts,bin,.system/{manifest,installed,logs}}
mkdir -p "${HOME}"/.local "${HOME}"/.config
chown -R developer:developer "${HOME}" 2>/dev/null || true

print_success "Workspace structure created"
