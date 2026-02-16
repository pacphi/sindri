#!/usr/bin/env bash
set -euo pipefail

# NotebookLM MCP CLI uninstaller

PACKAGE="notebooklm-mcp-cli"

echo "Uninstalling ${PACKAGE}..."

if command -v uv &>/dev/null; then
  uv tool uninstall "${PACKAGE}" 2>/dev/null || true
elif command -v pipx &>/dev/null; then
  pipx uninstall "${PACKAGE}" 2>/dev/null || true
elif command -v pip &>/dev/null; then
  pip uninstall -y "${PACKAGE}" 2>/dev/null || true
fi

echo "Successfully uninstalled ${PACKAGE}"
