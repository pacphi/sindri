#!/usr/bin/env bash
set -euo pipefail

# Claude CodePro v4.5.29 Installer for Sindri
# Downloads and executes the official installer

VERSION="v4.5.29"
INSTALLER_URL="https://raw.githubusercontent.com/maxritter/claude-codepro/${VERSION}/install.sh"

echo "===> Installing Claude CodePro ${VERSION}"
echo ""
echo "⚠️  IMPORTANT NOTES:"
echo "   - Claude CodePro requires a license (free tier available)"
echo "   - It will take FULL CONTROL of the .claude/ directory"
echo "   - Incompatible with claude-flow-v3, agentic-flow, agentic-qe"
echo ""

# Check for conflicts
if [[ -d "$HOME/.claude" ]]; then
    if [[ -f "$HOME/.claude/bin/ccp" ]]; then
        echo "ℹ️  Claude CodePro already detected, skipping installation"
        exit 0
    fi

    # Check for conflicting extensions
    if [[ -f "$HOME/.claude/config.json" ]] || [[ -d "$HOME/.claude/swarm.state" ]] || [[ -d "$HOME/.agentic-flow" ]] || [[ -d "$HOME/.agentic-qe" ]]; then
        echo "❌ ERROR: Conflicting Claude extensions detected!"
        echo ""
        echo "Claude CodePro is incompatible with:"
        echo "  - claude-flow-v3 (detected .claude/config.json)"
        echo "  - agentic-flow (detected .agentic-flow)"
        echo "  - agentic-qe (detected .agentic-qe)"
        echo ""
        echo "Please remove conflicting extensions before installing Claude CodePro."
        exit 1
    fi
fi

# Download and execute installer
echo "===> Downloading installer from GitHub..."
curl -fsSL "${INSTALLER_URL}" -o /tmp/ccp-install.sh

echo "===> Running Claude CodePro installer..."
echo ""
bash /tmp/ccp-install.sh

# Clean up
rm -f /tmp/ccp-install.sh

echo ""
echo "===> Claude CodePro ${VERSION} installation initiated"
echo ""
echo "📝 Next steps:"
echo "   1. Register license: ccp register"
echo "   2. Initialize project: ccp setup"
echo ""
echo "📚 Documentation: https://github.com/maxritter/claude-codepro"
