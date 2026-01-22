#!/usr/bin/env bash
set -euo pipefail

# Uninstall Ralph Inferno
echo "Uninstalling Ralph Inferno..."

# Remove Ralph home directory
if [ -d "$HOME/.ralph" ]; then
    echo "Removing Ralph home directory..."
    rm -rf "$HOME/.ralph"
fi

# Remove project-specific .ralph directory if in a project
if [ -d ".ralph" ]; then
    echo "Removing project .ralph directory..."
    rm -rf ".ralph"
fi

echo "Ralph Inferno uninstalled successfully"
