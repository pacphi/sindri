#!/usr/bin/env bash
set -euo pipefail

# pal-mcp-server Installer for Sindri
# Installs the Provider Abstraction Layer MCP Server

# Find common.sh relative to this script's location
# Script is at: /opt/sindri/extensions/pal-mcp-server/scripts/install.sh
# common.sh is at: /opt/sindri/common.sh (go up 3 levels)
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source "$(dirname "$(dirname "$(dirname "$SCRIPT_DIR")")")/common.sh"

EXTENSION_NAME="pal-mcp-server"
EXTENSION_DIR="${HOME}/extensions/${EXTENSION_NAME}"
RESOURCE_DIR="$SCRIPT_DIR/resources"
REPO_URL="https://github.com/BeehiveInnovations/pal-mcp-server.git"
VERSION="v9.8.2"

print_status "Installing ${EXTENSION_NAME} ${VERSION}..."

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Clone or update repository
if [[ -d "${EXTENSION_DIR}/.git" ]]; then
    print_status "Updating existing installation..."
    cd "${EXTENSION_DIR}"
    git fetch origin
    git checkout "${VERSION}" 2>/dev/null || git checkout main
    git pull origin "$(git branch --show-current)"
else
    print_status "Cloning repository..."
    git clone --branch "${VERSION}" --depth 1 "${REPO_URL}" "${EXTENSION_DIR}"
fi

cd "${EXTENSION_DIR}"

# Copy SKILL.md if available
if [[ -f "${RESOURCE_DIR}/SKILL.md" ]]; then
    cp "${RESOURCE_DIR}/SKILL.md" "${EXTENSION_DIR}/"
fi

# Check Python version
if ! command -v python3 &> /dev/null; then
    print_error "Python 3 is required but not found"
    exit 1
fi

PYTHON_VERSION=$(python3 --version | awk '{print $2}' | cut -d. -f1,2)
PYTHON_MAJOR=$(echo "${PYTHON_VERSION}" | cut -d. -f1)
PYTHON_MINOR=$(echo "${PYTHON_VERSION}" | cut -d. -f2)

print_status "Found Python ${PYTHON_VERSION}"

if [[ "${PYTHON_MAJOR}" -lt 3 ]] || { [[ "${PYTHON_MAJOR}" -eq 3 ]] && [[ "${PYTHON_MINOR}" -lt 10 ]]; }; then
    print_error "Python 3.10+ required, found ${PYTHON_VERSION}"
    exit 1
fi

# Create virtual environment
print_status "Creating Python virtual environment..."
if [[ ! -d ".pal_venv" ]]; then
    python3 -m venv .pal_venv
    print_success "Virtual environment created"
else
    print_status "Virtual environment already exists"
fi

# Upgrade pip in venv
print_status "Upgrading pip..."
.pal_venv/bin/python -m pip install --upgrade pip > /dev/null 2>&1

# Install uv for faster dependency installation
print_status "Installing uv package installer..."
.pal_venv/bin/python -m pip install uv > /dev/null 2>&1 || {
    print_warning "uv installation failed, falling back to pip"
}

# Install dependencies
print_status "Installing Python dependencies (this may take 2-3 minutes)..."
if [[ -f ".pal_venv/bin/uv" ]]; then
    # Use uv with virtual environment
    VIRTUAL_ENV="${EXTENSION_DIR}/.pal_venv" .pal_venv/bin/uv pip install -r requirements.txt
else
    # Fallback to pip
    .pal_venv/bin/python -m pip install -r requirements.txt
fi

print_success "Dependencies installed"

# Create .env from .env.example if it doesn't exist
if [[ ! -f ".env" ]]; then
    if [[ -f ".env.example" ]]; then
        print_status "Creating .env from template..."
        cp .env.example .env
        print_success ".env created from template"
        print_warning "Configure API keys in ${EXTENSION_DIR}/.env before using"
    else
        print_warning ".env.example not found, creating minimal .env"
        cat > .env << 'EOF'
# PAL MCP Server Configuration
# Configure at least one API key for your preferred provider

# Google Gemini (recommended for large context)
# GEMINI_API_KEY=your_key_here

# OpenAI
# OPENAI_API_KEY=your_key_here

# X.AI Grok
# XAI_API_KEY=your_key_here

# OpenRouter (unified access to 50+ models)
# OPENROUTER_API_KEY=your_key_here

# Local models (Ollama - no API key needed)
# CUSTOM_API_URL=http://localhost:11434
EOF
    fi
fi

# Register with Claude Code
CLAUDE_SETTINGS="${HOME}/.claude/settings.json"
if [[ -f "${CLAUDE_SETTINGS}" ]]; then
    print_status "Registering with Claude Code..."

    # Check if already registered
    if grep -q '"pal"' "${CLAUDE_SETTINGS}" 2>/dev/null; then
        print_info "pal-mcp-server already registered in Claude Code"
    else
        # Use Python to safely merge JSON
        python3 << PYTHON
import json
import sys

try:
    with open('${CLAUDE_SETTINGS}', 'r') as f:
        settings = json.load(f)

    if 'mcpServers' not in settings:
        settings['mcpServers'] = {}

    settings['mcpServers']['pal'] = {
        'command': '${EXTENSION_DIR}/.pal_venv/bin/python',
        'args': ['${EXTENSION_DIR}/server.py'],
        'env': {}
    }

    with open('${CLAUDE_SETTINGS}', 'w') as f:
        json.dump(settings, f, indent=2)

    print('Successfully registered pal-mcp-server')
except Exception as e:
    print(f'Error: {e}', file=sys.stderr)
    sys.exit(1)
PYTHON

        if [[ $? -eq 0 ]]; then
            print_success "Registered pal-mcp-server with Claude Code"
        else
            print_warning "Failed to register with Claude Code (manual registration may be needed)"
        fi
    fi
else
    print_info "Claude Code settings not found at ${CLAUDE_SETTINGS}"
    print_info "MCP server will be available but not auto-registered"
fi

# Create installation metadata
cat > "${EXTENSION_DIR}/installation-info.json" << EOF
{
  "extension": "${EXTENSION_NAME}",
  "version": "${VERSION}",
  "installed_at": "$(date -Iseconds 2>/dev/null || date +%Y-%m-%dT%H:%M:%S)",
  "installation_method": "sindri-extension",
  "venv_path": "${EXTENSION_DIR}/.pal_venv",
  "server_path": "${EXTENSION_DIR}/server.py",
  "mcp_server_name": "pal"
}
EOF

print_success "Installed ${EXTENSION_NAME} ${VERSION}"
print_info "Location: ${EXTENSION_DIR}"
print_info "MCP Server: pal"
echo ""
print_info "Next steps:"
print_info "1. Configure API keys in ${EXTENSION_DIR}/.env"
print_info "2. Restart Claude Code to load the MCP server"
print_info "3. Available tools: chat, thinkdeep, planner, consensus, codereview, debug, clink, and more"
echo ""
print_info "Documentation: ${EXTENSION_DIR}/SKILL.md"
