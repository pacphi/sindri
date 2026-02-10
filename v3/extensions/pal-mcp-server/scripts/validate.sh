#!/usr/bin/env bash
set -euo pipefail

# pal-mcp-server Validation Script

# Find common.sh relative to this script's location
# Script is at: /opt/sindri/extensions/pal-mcp-server/scripts/validate.sh
# common.sh is at: /opt/sindri/common.sh (go up 3 levels)
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source "$(dirname "$(dirname "$(dirname "$SCRIPT_DIR")")")/common.sh"

EXTENSION_DIR="${HOME}/extensions/pal-mcp-server"

# Check if installation directory exists
if [[ ! -d "${EXTENSION_DIR}" ]]; then
    error "pal-mcp-server not found at ${EXTENSION_DIR}"
    exit 1
fi

# Check if virtual environment exists
if [[ ! -f "${EXTENSION_DIR}/.pal_venv/bin/python" ]]; then
    error "Python virtual environment not found"
    exit 1
fi

# Check if server.py exists
if [[ ! -f "${EXTENSION_DIR}/server.py" ]]; then
    error "server.py not found"
    exit 1
fi

# Check if .env exists
if [[ ! -f "${EXTENSION_DIR}/.env" ]]; then
    warning ".env file not found (API keys not configured)"
fi

# Verify Python can import required modules
cd "${EXTENSION_DIR}"
if ! .pal_venv/bin/python -c "import mcp; import google.genai" 2>/dev/null; then
    error "Required Python dependencies not installed"
    exit 1
fi

success "pal-mcp-server validation passed"
info "Location: ${EXTENSION_DIR}"
info "Python: $(.pal_venv/bin/python --version)"
info "MCP Server: pal"
