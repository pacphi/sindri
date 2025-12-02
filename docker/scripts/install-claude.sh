#!/bin/bash
# Install Claude Code CLI system-wide
# Uses Anthropic's official curl installer with timeout protection
#
# This script:
# 1. Creates a temporary home for the developer user
# 2. Runs the installer as developer user (with timeout)
# 3. Moves the binary to /usr/local/bin for system-wide access
# 4. Sets up /etc/skel/.claude/ for new user home directories
#
# Usage: install-claude.sh [--timeout SECONDS]
#   --timeout: Installation timeout in seconds (default: 300)

set -euo pipefail

# Source common utilities if available
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -f "$SCRIPT_DIR/../lib/common.sh" ]]; then
    source "$SCRIPT_DIR/../lib/common.sh"
else
    # Fallback print functions
    print_status() { echo "==> $1"; }
    print_success() { echo "[OK] $1"; }
    print_warning() { echo "[WARN] $1"; }
    print_error() { echo "[ERROR] $1" >&2; }
fi

# Configuration
DEVELOPER_USER="${DEV_USER:-developer}"
INSTALL_TIMEOUT=300
SYSTEM_BIN_DIR="/usr/local/bin"
SKEL_CLAUDE_DIR="/etc/skel/.claude"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --timeout)
            INSTALL_TIMEOUT="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

print_status "Installing Claude Code CLI..."

# Create a temporary home directory for the developer user during build
# (The real home will be on a volume at runtime)
TEMP_HOME=$(mktemp -d)
trap 'rm -rf "$TEMP_HOME"' EXIT

# Ensure developer user exists
if ! id "$DEVELOPER_USER" &>/dev/null; then
    print_error "Developer user '$DEVELOPER_USER' does not exist"
    exit 1
fi

# Set ownership of temp home
chown "$DEVELOPER_USER:$DEVELOPER_USER" "$TEMP_HOME"

print_status "Running Claude Code installer as $DEVELOPER_USER (timeout: ${INSTALL_TIMEOUT}s)..."

# Run the installer as developer user with timeout
# The installer places the binary in ~/.local/bin/claude
# Note: Use 'su' without '-' to avoid trying to cd to non-existent home directory
if ! su "$DEVELOPER_USER" -c "
    export HOME='$TEMP_HOME'
    export XDG_DATA_HOME='$TEMP_HOME/.local/share'
    export XDG_CONFIG_HOME='$TEMP_HOME/.config'
    export XDG_CACHE_HOME='$TEMP_HOME/.cache'
    mkdir -p '$TEMP_HOME/.local/bin'
    timeout $INSTALL_TIMEOUT bash -c 'set -o pipefail; curl -fsSL https://claude.ai/install.sh | bash'
"; then
    print_error "Claude Code installation failed or timed out"
    exit 1
fi

# Find the installed binary
CLAUDE_USER_PATH="$TEMP_HOME/.local/bin/claude"
if [[ ! -f "$CLAUDE_USER_PATH" ]]; then
    # Try alternate location
    CLAUDE_USER_PATH="$TEMP_HOME/.claude/local/bin/claude"
fi

if [[ ! -f "$CLAUDE_USER_PATH" ]]; then
    print_error "Claude binary not found after installation"
    print_status "Contents of temp home:"
    find "$TEMP_HOME" -type f -name "claude*" 2>/dev/null || true
    exit 1
fi

# Copy to system-wide location (use cp -L to follow symlinks)
print_status "Installing Claude Code to $SYSTEM_BIN_DIR/claude..."
cp -L "$CLAUDE_USER_PATH" "$SYSTEM_BIN_DIR/claude"
chmod +x "$SYSTEM_BIN_DIR/claude"

# Verify installation
if ! "$SYSTEM_BIN_DIR/claude" --version &>/dev/null; then
    print_warning "Claude installed but version check failed (may need runtime dependencies)"
fi

# Create /etc/skel/.claude directory for new user homes
print_status "Setting up /etc/skel/.claude for new users..."
mkdir -p "$SKEL_CLAUDE_DIR"

# Create a minimal settings file that will be copied to new home directories
cat > "$SKEL_CLAUDE_DIR/settings.json" << 'EOF'
{
  "permissions": {
    "allow": [],
    "deny": []
  },
  "env": {}
}
EOF

# Set proper permissions on skel directory
chmod 755 "$SKEL_CLAUDE_DIR"
chmod 644 "$SKEL_CLAUDE_DIR/settings.json"

print_success "Claude Code installed: $SYSTEM_BIN_DIR/claude"
print_status "Users will have ~/.claude/ created from /etc/skel on first login"
