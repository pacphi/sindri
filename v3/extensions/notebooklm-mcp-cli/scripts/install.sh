#!/usr/bin/env bash
set -euo pipefail

# NotebookLM MCP CLI installer
# Installs notebooklm-mcp-cli via uv (preferred), pipx, or pip

PACKAGE="notebooklm-mcp-cli"

echo "Installing ${PACKAGE}..."

if command -v uv &>/dev/null; then
  echo "Using uv to install ${PACKAGE}"
  uv tool install "${PACKAGE}"
elif command -v pipx &>/dev/null; then
  echo "Using pipx to install ${PACKAGE}"
  pipx install "${PACKAGE}"
elif command -v pip &>/dev/null; then
  echo "Using pip to install ${PACKAGE}"
  pip install --user "${PACKAGE}"
else
  echo "ERROR: No Python package installer found (uv, pipx, or pip required)" >&2
  exit 1
fi

# Ensure ~/.local/bin is in PATH (uv/pipx install location)
export PATH="${HOME}/.local/bin:${PATH}"

# Verify installation
if command -v nlm &>/dev/null; then
  echo "Successfully installed ${PACKAGE}"
  nlm --version
else
  echo "WARNING: nlm command not found in PATH after installation" >&2
  echo "You may need to restart your shell or add the install location to PATH" >&2
  exit 1
fi
