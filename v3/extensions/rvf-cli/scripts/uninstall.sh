#!/usr/bin/env bash
set -euo pipefail

echo "Removing rvf-cli..."

if command -v cargo &>/dev/null; then
    cargo uninstall rvf-cli 2>/dev/null || true
fi

echo "rvf-cli removed successfully."
