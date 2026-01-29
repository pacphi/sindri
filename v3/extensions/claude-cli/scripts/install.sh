#!/bin/bash
# ==============================================================================
# Claude Code CLI Installer for Sindri v3
# ==============================================================================
# Installs Claude Code CLI using Anthropic's official installer
#
# This script:
# 1. Downloads and runs the official installer from claude.ai
# 2. Installs to ~/.local/bin/claude
# 3. Sets up ~/.claude/ configuration directory
#
# Based on: v2/docker/scripts/install-claude.sh
# ==============================================================================

set -euo pipefail

# Configuration
INSTALL_TIMEOUT="${INSTALL_TIMEOUT:-300}"
CLAUDE_INSTALL_URL="https://claude.ai/install.sh"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_status() { echo -e "${YELLOW}==>${NC} $1"; }
print_success() { echo -e "${GREEN}[OK]${NC} $1"; }
print_error() { echo -e "${RED}[ERROR]${NC} $1" >&2; }

# Determine home directory (respect HOME env var for Docker compatibility)
HOME_DIR="${HOME:-$(eval echo ~)}"
LOCAL_BIN="$HOME_DIR/.local/bin"
CLAUDE_DIR="$HOME_DIR/.claude"

print_status "Installing Claude Code CLI..."
print_status "Home directory: $HOME_DIR"
print_status "Install location: $LOCAL_BIN/claude"

# Ensure directories exist
mkdir -p "$LOCAL_BIN"
mkdir -p "$CLAUDE_DIR"

# Check if already installed and working
if command -v claude &>/dev/null; then
    CURRENT_VERSION=$(claude --version 2>/dev/null || echo "unknown")
    print_status "Claude Code already installed: $CURRENT_VERSION"
    print_status "Reinstalling to ensure latest version..."
fi

# Download and run the official installer with timeout
print_status "Running Claude Code installer (timeout: ${INSTALL_TIMEOUT}s)..."

# The installer expects these environment variables
export XDG_DATA_HOME="${XDG_DATA_HOME:-$HOME_DIR/.local/share}"
export XDG_CONFIG_HOME="${XDG_CONFIG_HOME:-$HOME_DIR/.config}"
export XDG_CACHE_HOME="${XDG_CACHE_HOME:-$HOME_DIR/.cache}"

# Run installer with timeout and capture output
if ! timeout "$INSTALL_TIMEOUT" bash -c "set -o pipefail; curl -fsSL '$CLAUDE_INSTALL_URL' | bash" 2>&1; then
    print_error "Claude Code installation failed or timed out"
    print_error "Please check your network connection and try again"
    exit 1
fi

# Find the installed binary
CLAUDE_PATH=""
for path in "$LOCAL_BIN/claude" "$HOME_DIR/.claude/local/bin/claude"; do
    if [[ -f "$path" ]]; then
        CLAUDE_PATH="$path"
        break
    fi
done

if [[ -z "$CLAUDE_PATH" ]]; then
    print_error "Claude binary not found after installation"
    print_status "Searching for claude binary..."
    find "$HOME_DIR" -type f -name "claude*" 2>/dev/null | head -5 || true
    exit 1
fi

# Ensure it's in the expected location
if [[ "$CLAUDE_PATH" != "$LOCAL_BIN/claude" ]]; then
    print_status "Moving claude to $LOCAL_BIN/claude..."
    cp -L "$CLAUDE_PATH" "$LOCAL_BIN/claude"
    chmod +x "$LOCAL_BIN/claude"
fi

# Verify installation
if "$LOCAL_BIN/claude" --version &>/dev/null; then
    VERSION=$("$LOCAL_BIN/claude" --version 2>&1 | head -1)
    print_success "Claude Code installed successfully: $VERSION"
else
    print_error "Claude installed but version check failed"
    print_status "The binary may need runtime dependencies - try running 'claude --version' manually"
    # Don't fail - it might work at runtime
fi

# Create default settings if they don't exist
SETTINGS_FILE="$CLAUDE_DIR/settings.json"
if [[ ! -f "$SETTINGS_FILE" ]]; then
    print_status "Creating default settings at $SETTINGS_FILE..."
    cat > "$SETTINGS_FILE" << 'SETTINGS_EOF'
{
  "permissions": {
    "allow": [],
    "deny": []
  },
  "env": {}
}
SETTINGS_EOF
    chmod 644 "$SETTINGS_FILE"
fi

print_success "Claude Code CLI installation complete"
print_status "Run 'claude' to start using Claude Code"
