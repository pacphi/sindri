#!/usr/bin/env bash
set -euo pipefail

echo "Installing ruvector-cli via cargo..."

# Ensure cargo is available
if ! command -v cargo &>/dev/null; then
    echo "ERROR: cargo not found. Please install the rust extension first." >&2
    exit 1
fi

cargo install ruvector-cli

echo "ruvector-cli installed successfully."
