#!/bin/bash
set -euo pipefail
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Installing agent-manager..."

GITHUB_REPO="pacphi/claude-code-agent-manager"
BINARY_NAME="agent-manager"
INSTALL_PATH="$HOME/.local/bin"

# Check if already installed
if [[ -f "${INSTALL_PATH}/${BINARY_NAME}" ]]; then
    if "${INSTALL_PATH}/${BINARY_NAME}" version >/dev/null 2>&1; then
        current_version=$("${INSTALL_PATH}/${BINARY_NAME}" version 2>/dev/null | head -n1 || echo "unknown")
        print_warning "agent-manager is already installed: $current_version"
        print_status "Skipping installation (remove manually to reinstall)"
        exit 0
    fi
fi

# Verify prerequisites
if ! command -v curl >/dev/null 2>&1; then
    print_error "curl is required but not installed"
    print_status "Install with: sudo apt-get install curl"
    exit 1
fi

# Get latest release using standardized GitHub release version detection
# Uses gh CLI with curl fallback for reliability
# Note: agent-manager uses prereleases, so we pass true for include_prereleases
print_status "Fetching latest release (including prereleases)..."
tag_name=$(get_github_release_version "${GITHUB_REPO}" true true)

if [[ -z "$tag_name" ]]; then
    print_warning "Could not fetch latest release, using fallback v1.0.0"
    tag_name="v1.0.0"
fi

print_status "Latest release: $tag_name"

# Detect platform
case "$(uname -s)-$(uname -m)" in
    Linux-x86_64|Linux-amd64)
        platform_arch="linux-amd64"
        ;;
    Linux-aarch64|Linux-arm64)
        platform_arch="linux-arm64"
        ;;
    Darwin-x86_64|Darwin-amd64)
        platform_arch="darwin-amd64"
        ;;
    Darwin-arm64|Darwin-aarch64)
        platform_arch="darwin-arm64"
        ;;
    *)
        print_error "Unsupported platform: $(uname -s)-$(uname -m)"
        exit 1
        ;;
esac

print_status "Detected platform: $platform_arch"

# Download binary
binary_name="${BINARY_NAME}-${platform_arch}"
download_url="https://github.com/${GITHUB_REPO}/releases/download/${tag_name}/${binary_name}"

print_status "Downloading agent-manager binary..."
mkdir -p "$INSTALL_PATH"

if curl -L -o "${INSTALL_PATH}/${BINARY_NAME}" "$download_url"; then
    print_success "Binary downloaded successfully"
else
    print_error "Failed to download binary"
    exit 1
fi

# Make executable
chmod +x "${INSTALL_PATH}/${BINARY_NAME}"

# Verify installation
if "${INSTALL_PATH}/${BINARY_NAME}" version >/dev/null 2>&1; then
    version=$("${INSTALL_PATH}/${BINARY_NAME}" version 2>/dev/null | head -n1 || echo "unknown")
    print_success "Agent Manager installed successfully: $version"
else
    print_error "Agent Manager installation failed - binary not working"
    exit 1
fi

print_success "Agent manager installation complete"
