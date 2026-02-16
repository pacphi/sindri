#!/usr/bin/env bash
set -euo pipefail

echo "Removing ruvector-cli..."

if command -v cargo &>/dev/null; then
    cargo uninstall ruvector-cli 2>/dev/null || true
fi

echo "ruvector-cli removed successfully."
