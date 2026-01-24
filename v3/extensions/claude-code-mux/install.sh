#!/bin/bash
set -euo pipefail

# claude-code-mux install script
# Installs CCM binary from GitHub releases

# Source common utilities
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Installing Claude Code Mux (CCM)..."

# Detect architecture
ARCH=$(uname -m)
OS=$(uname -s | tr '[:upper:]' '[:lower:]')

# Map architecture names
case "$ARCH" in
    x86_64|amd64)
        ARCH="x86_64"
        ;;
    aarch64|arm64)
        ARCH="aarch64"
        ;;
    *)
        print_error "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

# Determine asset name based on OS and architecture
case "$OS" in
    linux)
        # Linux ARM64 builds are not available from upstream
        if [[ "$ARCH" == "aarch64" ]]; then
            print_warning "Claude Code Mux does not provide Linux ARM64 binaries"
            print_warning "See: https://github.com/9j/claude-code-mux/releases"
            print_status "Skipping claude-code-mux installation on this platform"
            exit 0  # Exit cleanly to not fail the profile
        fi
        # Use musl for broader compatibility on x86_64
        ASSET="ccm-linux-${ARCH}-musl.tar.gz"
        ;;
    darwin)
        ASSET="ccm-macos-${ARCH}.tar.gz"
        ;;
    *)
        print_error "Unsupported operating system: $OS"
        exit 1
        ;;
esac

print_status "Detected platform: $OS-$ARCH"
print_status "Downloading asset: $ASSET"

# Get latest version using standardized GitHub release version detection
# Uses gh CLI with curl fallback for reliability
CCM_VERSION=$(get_github_release_version "9j/claude-code-mux" true)
if [[ -z "$CCM_VERSION" ]]; then
    print_error "Failed to determine latest CCM version from GitHub"
    exit 1
fi
print_status "Latest CCM version: $CCM_VERSION"

# Download and extract CCM from specific release (more reliable than /latest/download)
DOWNLOAD_URL="https://github.com/9j/claude-code-mux/releases/download/${CCM_VERSION}/${ASSET}"
TMP_DIR=$(mktemp -d)

trap 'rm -rf "$TMP_DIR"' EXIT

print_status "Downloading from: $DOWNLOAD_URL"
if ! curl -fsSL "$DOWNLOAD_URL" -o "$TMP_DIR/ccm.tar.gz"; then
    print_error "Failed to download CCM binary"
    exit 1
fi

print_status "Extracting binary..."
if ! tar -xzf "$TMP_DIR/ccm.tar.gz" -C "$TMP_DIR"; then
    print_error "Failed to extract CCM binary"
    exit 1
fi

# Install to workspace bin directory
BIN_DIR="${WORKSPACE:-${HOME}/workspace}/bin"
mkdir -p "$BIN_DIR"
mv "$TMP_DIR/ccm" "$BIN_DIR/ccm"
chmod +x "$BIN_DIR/ccm"

# Verify installation
if "$BIN_DIR/ccm" --version >/dev/null 2>&1; then
    VERSION=$("$BIN_DIR/ccm" --version 2>&1 | head -n1)
    print_success "CCM installed successfully: $VERSION"
else
    print_error "CCM installation verification failed"
    exit 1
fi

# Initialize CCM configuration (creates ~/.claude-code-mux/config.toml)
print_status "Initializing CCM configuration..."
if ! timeout 5 "$BIN_DIR/ccm" start >/dev/null 2>&1 & then
    print_warning "CCM auto-configuration may require manual setup"
fi
sleep 2
pkill -f "ccm start" || true

if [[ -f ~/.claude-code-mux/config.toml ]]; then
    print_success "CCM configuration initialized at ~/.claude-code-mux/config.toml"
else
    print_warning "CCM config not auto-created. Will be generated on first run."
fi

print_status ""
print_status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
print_success "Claude Code Mux installation complete!"
print_status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
print_status ""
print_status "🚀 QUICKSTART (Recommended)"
print_status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
print_status ""
print_status "  Run the interactive setup wizard for multi-model routing:"
print_status ""
print_success "    ccm-quickstart"
print_status ""
print_status "  Choose from ready-to-use configurations:"
print_status "    • Free OAuth (Claude Pro/Max + ChatGPT Plus)"
print_status "    • API Key with automatic failover"
print_status "    • Cost-optimized multi-provider routing"
print_status "    • Custom setup with template guidance"
print_status ""
print_status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
print_status "📚 MANUAL SETUP"
print_status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
print_status ""
print_status "  Server Management:"
print_status "    ccmctl start           Start CCM server"
print_status "    ccmctl stop            Stop server"
print_status "    ccmctl restart         Restart server"
print_status "    ccmctl status          Check status"
print_status "    ccmctl logs            View logs"
print_status ""
print_status "  Configuration:"
print_status "    Web UI:                http://127.0.0.1:13456 (easiest)"
print_status "    Config file:           ~/workspace/config/ccm-config.toml"
print_status "    Edit config:           \$EDITOR ~/workspace/config/ccm-config.toml"
print_status ""
print_status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
print_status "⚡ HOW IT WORKS"
print_status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
print_status ""
print_status "  1. CCM proxies AI requests across 18+ providers (Anthropic, OpenAI, etc.)"
print_status "  2. Automatic failover when primary provider has outages"
print_status "  3. Route by task type: websearch → Gemini, reasoning → Claude, etc."
print_status "  4. ~5MB RAM, <1ms overhead, full streaming support"
print_status ""
print_status "  Claude Code environment variables (already configured):"
print_status "    ANTHROPIC_BASE_URL=http://127.0.0.1:13456"
print_status "    ANTHROPIC_API_KEY=ccm-proxy"
print_status ""
print_status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
print_success "Next: ccm-quickstart"
print_status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
