#!/bin/bash
set -euo pipefail

# Uninstall rust using rustup
if command -v rustup &> /dev/null; then
  rustup self uninstall -y || true
fi

# Clean up cargo and rustup directories
rm -rf "${HOME}/.cargo" "${HOME}/.rustup" "${HOME}/.cache/tmp"

# Remove PATH entry from profile
sed -i '/\.cargo\/bin/d' "${HOME}/.profile" 2>/dev/null || true

echo "Rust uninstalled successfully"
