#!/usr/bin/env bash
set -euo pipefail

# Non-interactive Ralph Inferno project initialization
# This script is called by new-project and clone-project via project-init capability

PROJECT_DIR="${PWD}"
RALPH_DIR="${PROJECT_DIR}/.ralph"
CONFIG_FILE="${RALPH_DIR}/config.json"

# Check if already initialized (idempotent)
if [[ -f "$CONFIG_FILE" ]]; then
    echo "✓ Ralph Inferno already initialized"
    exit 0
fi

echo "Initializing Ralph Inferno..."

# Create .ralph directory
mkdir -p "$RALPH_DIR"

# Detect GitHub username
GITHUB_USER=$(git config user.name 2>/dev/null || echo "${USER:-developer}")

# Detect ralph-inferno version
RALPH_VERSION=$(npm view ralph-inferno version 2>/dev/null || echo "1.0.6")

# Create config.json with sensible defaults
cat > "$CONFIG_FILE" <<EOF
{
  "version": "${RALPH_VERSION}",
  "language": "en",
  "provider": "none",
  "github": {
    "username": "${GITHUB_USER}"
  },
  "claude": {
    "auth_method": "subscription"
  },
  "notifications": {
    "ntfy_enabled": false
  },
  "user": "${USER:-developer}"
}
EOF

echo "✓ Created configuration: .ralph/config.json"

# Run ralph-inferno update to copy all files (non-interactive!)
echo "Installing Ralph Inferno files..."
if npx ralph-inferno update; then
    echo "✓ Ralph Inferno files installed"
else
    echo "⚠️  Ralph update failed - files may be incomplete"
    exit 1
fi

# Create ralph wrapper at project root (in case update didn't create it)
if [[ ! -f "${PROJECT_DIR}/ralph" ]]; then
    cat > "${PROJECT_DIR}/ralph" <<'WRAPPER'
#!/bin/bash
# Ralph CLI wrapper
RALPH_DIR=".ralph"
exec "$RALPH_DIR/scripts/ralph.sh" "$@"
WRAPPER
    chmod +x "${PROJECT_DIR}/ralph"
    echo "✓ Created ralph wrapper"
fi

echo ""
echo "╔═══════════════════════════════════════════════════════════╗"
echo "║                                                           ║"
echo "║              🔥 RALPH INFERNO INSTALLED! 🔥               ║"
echo "║                                                           ║"
echo "╚═══════════════════════════════════════════════════════════╝"
echo ""
echo "Next steps:"
echo "  1. Run /discover in Claude Code to set up your project"
echo "  2. Or run: ./ralph --help"
echo ""
echo "⚠️  Configuration is set to 'no VM' - Ralph runs in this container."
echo "   To customize (add VM, change settings): npx ralph-inferno install"
