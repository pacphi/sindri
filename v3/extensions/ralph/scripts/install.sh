#!/usr/bin/env bash
set -euo pipefail

# Install Ralph Inferno
echo "Installing Ralph Inferno..."

# Ensure Node.js is available
if ! command -v node &> /dev/null; then
    echo "Error: Node.js is required but not found. Install the nodejs extension first."
    exit 1
fi

# Ensure npm/npx is available
if ! command -v npx &> /dev/null; then
    echo "Error: npx is required but not found. Install the nodejs extension first."
    exit 1
fi

# Create Ralph home directory
mkdir -p "$HOME/.ralph"

echo "Ralph Inferno installed successfully"
echo ""
echo "Next steps:"
echo "  1. Navigate to a project directory (or use clone-project/new-project)"
echo "  2. Project initialization will happen automatically via project-init capability"
echo "  3. Alternatively, run: npx ralph-inferno install (for manual setup)"
echo ""
echo "⚠️  IMPORTANT: Always run Ralph on a disposable VM, never on your local machine."
