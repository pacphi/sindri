#!/usr/bin/env bash
set -euo pipefail

# Validate Ralph Inferno installation
echo "Validating Ralph Inferno installation..."

# Check Node.js
if ! command -v node &> /dev/null; then
    echo "Error: Node.js not found"
    exit 1
fi

# Check npx
if ! command -v npx &> /dev/null; then
    echo "Error: npx not found"
    exit 1
fi

# Check Ralph home directory
if [ ! -d "$HOME/.ralph" ]; then
    echo "Warning: Ralph home directory not found at $HOME/.ralph"
fi

echo "Ralph Inferno validation successful"
