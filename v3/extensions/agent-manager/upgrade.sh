#!/bin/bash
set -euo pipefail
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Upgrading agent-manager..."

GITHUB_REPO="pacphi/claude-code-agent-manager"
BINARY_NAME="agent-manager"
INSTALL_PATH="$HOME/.local/bin"

# Check if installed
if [[ ! -f "${INSTALL_PATH}/${BINARY_NAME}" ]]; then
    print_error "agent-manager is not installed. Install it first."
    exit 1
fi

# Get current version
if "${INSTALL_PATH}/${BINARY_NAME}" version >/dev/null 2>&1; then
    current_version=$("${INSTALL_PATH}/${BINARY_NAME}" version 2>/dev/null | head -n1 || echo "unknown")
    print_status "Current version: $current_version"
else
    print_warning "Could not determine current version"
    current_version="unknown"
fi

# Verify prerequisites
if ! command -v curl >/dev/null 2>&1; then
    print_error "curl is required but not installed"
    exit 1
fi

# Get latest release using standardized GitHub release version detection
# Uses gh CLI with curl fallback for reliability
# Note: agent-manager uses prereleases, so we pass true for include_prereleases
print_status "Fetching latest release (including prereleases)..."
tag_name=$(get_github_release_version "${GITHUB_REPO}" true true)

if [[ -z "$tag_name" ]]; then
    print_error "Could not fetch latest release from GitHub"
    exit 1
fi

print_status "Latest release: $tag_name"

# Check if upgrade is needed
if [[ "$current_version" == *"$tag_name"* ]] || [[ "$current_version" == "${tag_name#v}" ]]; then
    print_success "agent-manager is already at the latest version"
    exit 0
fi

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

# Download new version
binary_name="${BINARY_NAME}-${platform_arch}"
download_url="https://github.com/${GITHUB_REPO}/releases/download/${tag_name}/${binary_name}"

print_status "Downloading new version..."
temp_binary="${INSTALL_PATH}/${BINARY_NAME}.new"

if curl -L -o "$temp_binary" "$download_url"; then
    print_success "Binary downloaded successfully"
else
    print_error "Failed to download new version"
    rm -f "$temp_binary"
    exit 1
fi

# Make executable
chmod +x "$temp_binary"

# Verify new binary works
if "$temp_binary" version >/dev/null 2>&1; then
    new_version=$("$temp_binary" version 2>/dev/null | head -n1 || echo "unknown")
    print_status "New version: $new_version"
else
    print_error "Downloaded binary is not working"
    rm -f "$temp_binary"
    exit 1
fi

# Replace old binary with new one
print_status "Installing new version..."
mv "$temp_binary" "${INSTALL_PATH}/${BINARY_NAME}"

# Final verification
if "${INSTALL_PATH}/${BINARY_NAME}" version >/dev/null 2>&1; then
    final_version=$("${INSTALL_PATH}/${BINARY_NAME}" version 2>/dev/null | head -n1 || echo "unknown")
    print_success "Agent Manager upgraded successfully: $current_version → $final_version"
else
    print_error "Upgrade failed - binary not working"
    exit 1
fi

print_success "Agent manager upgrade complete"
