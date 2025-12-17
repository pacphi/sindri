#!/usr/bin/env bash
set -euo pipefail

# Install script for supabase-cli
# Installs Supabase CLI via .deb package from GitHub releases
# Note: npm global install is NOT supported by Supabase as of 2025

source "${DOCKER_LIB:-/docker/lib}/common.sh"

EXTENSION_DIR="${HOME}/extensions/supabase-cli"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/supabase-cli/resources"

print_status "Installing Supabase CLI..."

# Check if already installed
if command_exists supabase; then
    current_version=$(supabase --version 2>/dev/null || echo "unknown")
    print_warning "Supabase CLI already installed: $current_version"
    print_status "To upgrade, remove first with: extension-manager remove supabase-cli"
    exit 0
fi

# Detect architecture
ARCH=$(uname -m)
case "$ARCH" in
    x86_64|amd64)
        DEB_ARCH="amd64"
        ;;
    aarch64|arm64)
        DEB_ARCH="arm64"
        ;;
    *)
        print_error "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

# Get latest version using gh CLI (most reliable) or GitHub API
print_status "Fetching latest Supabase CLI version..."

LATEST_VERSION=""

# Method 1: Try gh CLI (most reliable, handles auth automatically)
if command_exists gh; then
    print_status "Attempting to fetch version using gh CLI..."
    LATEST_VERSION=$(gh release view --repo supabase/cli --json tagName --jq '.tagName' 2>/dev/null | sed 's/^v//' || echo "")
    if [[ -n "$LATEST_VERSION" ]]; then
        print_status "Latest version (via gh): v$LATEST_VERSION"
    fi
fi

# Method 2: Fall back to GitHub API with curl
if [[ -z "$LATEST_VERSION" ]]; then
    print_status "gh CLI failed, trying GitHub API..."

    # Use GitHub token for authentication if available
    CURL_AUTH_HEADER=""
    if [[ -n "${GITHUB_TOKEN:-}" ]]; then
        CURL_AUTH_HEADER="Authorization: Bearer ${GITHUB_TOKEN}"
    fi

    # Extract tag_name value (portable solution for BSD/GNU grep)
    if [[ -n "$CURL_AUTH_HEADER" ]]; then
        LATEST_VERSION=$(curl -fsSL -H "$CURL_AUTH_HEADER" "https://api.github.com/repos/supabase/cli/releases/latest" 2>/dev/null | \
            grep -o '"tag_name":"[^"]*"' | cut -d'"' -f4 | sed 's/^v//' || echo "")
    else
        LATEST_VERSION=$(curl -fsSL "https://api.github.com/repos/supabase/cli/releases/latest" 2>/dev/null | \
            grep -o '"tag_name":"[^"]*"' | cut -d'"' -f4 | sed 's/^v//' || echo "")
    fi

    if [[ -n "$LATEST_VERSION" ]]; then
        print_status "Latest version (via API): v$LATEST_VERSION"
    fi
fi

# Fail if both methods failed
if [[ -z "$LATEST_VERSION" ]]; then
    print_error "Failed to fetch latest version from GitHub"
    print_status "Both gh CLI and GitHub API failed"
    print_status "Check:"
    print_status "  1. Network connectivity: curl -I https://api.github.com/repos/supabase/cli/releases/latest"
    print_status "  2. GITHUB_TOKEN is set: echo \${GITHUB_TOKEN:0:8}..."
    print_status "  3. gh CLI is working: gh release view --repo supabase/cli"
    exit 1
fi

# Download .deb package
DEB_URL="https://github.com/supabase/cli/releases/download/v${LATEST_VERSION}/supabase_${LATEST_VERSION}_linux_${DEB_ARCH}.deb"
DEB_FILE="/tmp/supabase_${LATEST_VERSION}_linux_${DEB_ARCH}.deb"

print_status "Downloading Supabase CLI .deb package..."
if ! curl -fsSL -o "$DEB_FILE" "$DEB_URL"; then
    print_error "Failed to download Supabase CLI from: $DEB_URL"
    rm -f "$DEB_FILE"
    exit 1
fi

# Verify download
if [[ ! -f "$DEB_FILE" ]] || [[ ! -s "$DEB_FILE" ]]; then
    print_error "Downloaded file is empty or missing"
    rm -f "$DEB_FILE"
    exit 1
fi

# Install using dpkg (requires sudo)
print_status "Installing .deb package (requires sudo)..."
if sudo DEBIAN_FRONTEND=noninteractive dpkg -i "$DEB_FILE"; then
    print_success "Supabase CLI installed successfully"
else
    # dpkg might fail due to missing dependencies, try to fix
    print_warning "dpkg install had issues, attempting to fix dependencies..."
    sudo DEBIAN_FRONTEND=noninteractive apt-get install -f -y -qq
    # Retry dpkg
    if ! sudo DEBIAN_FRONTEND=noninteractive dpkg -i "$DEB_FILE"; then
        print_error "Failed to install Supabase CLI .deb package"
        rm -f "$DEB_FILE"
        exit 1
    fi
fi

# Clean up
rm -f "$DEB_FILE"

# Create extension directory and copy resources
mkdir -p "${EXTENSION_DIR}"
if [[ -d "${RESOURCE_DIR}" ]]; then
    cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/" 2>/dev/null || true
fi

# Verify installation
if command_exists supabase; then
    VERSION=$(supabase --version 2>/dev/null || echo "unknown")
    print_success "Supabase CLI v${VERSION} installed successfully"
else
    print_error "Supabase CLI installation failed - binary not found"
    exit 1
fi

print_status "Usage: supabase <command>"
print_status "Run 'supabase init' to initialize a new project"
print_status "Run 'supabase start' to start local Supabase services (requires Docker)"

if [[ -n "${SUPABASE_ACCESS_TOKEN:-}" ]]; then
    print_success "SUPABASE_ACCESS_TOKEN is configured"
else
    print_warning "SUPABASE_ACCESS_TOKEN not set - some features may be limited"
    print_status "Get your token from: https://supabase.com/dashboard/account/tokens"
fi
