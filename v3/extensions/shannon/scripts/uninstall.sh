#!/usr/bin/env bash
set -euo pipefail

# Shannon uninstallation script for Sindri V3

SHANNON_HOME="${HOME}/.shannon"

echo "Uninstalling Shannon..."

# Stop any running Shannon containers
if [ -d "${SHANNON_HOME}/shannon" ]; then
    echo "Stopping Shannon containers..."
    cd "${SHANNON_HOME}/shannon"
    ./shannon stop CLEAN=true 2>/dev/null || true
fi

# Remove Shannon directory
if [ -d "${SHANNON_HOME}" ]; then
    echo "Removing Shannon directory..."
    rm -rf "${SHANNON_HOME}"
fi

echo "Shannon uninstalled successfully."
echo ""
echo "Note: Docker images have been preserved."
echo "To remove Shannon Docker images, run:"
echo "  docker images | grep shannon | awk '{print \$3}' | xargs docker rmi"
