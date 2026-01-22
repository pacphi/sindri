#!/usr/bin/env bash
set -euo pipefail

# spec-kit doesn't require installation - it's invoked via uvx
# This script validates that uvx is available

if ! command -v uvx &>/dev/null; then
    echo "ERROR: uvx not found. Install the 'python' extension first."
    exit 1
fi

echo "spec-kit prerequisites satisfied (uvx available)"
exit 0
