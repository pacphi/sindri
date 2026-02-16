#!/usr/bin/env bash
set -euo pipefail

echo "Installing rvf-cli via cargo..."

# Ensure cargo is available
if ! command -v cargo &>/dev/null; then
    echo "ERROR: cargo not found. Please install the rust extension first." >&2
    exit 1
fi

# Use home directory for cargo build artifacts to avoid /tmp noexec issues
export CARGO_TARGET_DIR="${HOME}/.cache/cargo-build"
mkdir -p "$CARGO_TARGET_DIR"

cargo install rvf-cli

# Clean up build artifacts to reclaim disk space
rm -rf "$CARGO_TARGET_DIR"

echo "rvf-cli installed successfully."
