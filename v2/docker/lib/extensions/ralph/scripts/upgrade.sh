#!/usr/bin/env bash
set -euo pipefail

# Upgrade Ralph Inferno
echo "Upgrading Ralph Inferno..."

# Ralph is upgraded via npx, which always uses the latest version
echo "Ralph Inferno uses npx for execution, which automatically uses the latest version."
echo "No manual upgrade required."
echo ""
echo "To update project configuration, run:"
echo "  npx ralph-inferno install"
