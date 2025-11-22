#!/bin/bash
set -euo pipefail

# Source common utilities
source /docker/lib/common.sh

print_status "Installing ruvnet-aliases extension"

# Ensure we're in workspace directory
cd /workspace || exit 1

# Check if alias files exist in extension directory
EXTENSION_DIR="/docker/lib/extensions/ruvnet-aliases"
if [[ ! -f "$EXTENSION_DIR/agentic-flow.aliases" ]]; then
    print_error "agentic-flow.aliases not found in $EXTENSION_DIR"
    exit 1
fi

if [[ ! -f "$EXTENSION_DIR/claude-flow.aliases" ]]; then
    print_error "claude-flow.aliases not found in $EXTENSION_DIR"
    exit 1
fi

print_success "ruvnet-aliases extension installed successfully"
print_status "Aliases will be loaded from ~/.bashrc on next shell session"
