#!/usr/bin/env bash
set -euo pipefail

# Install script for supabase-cli
# Installs Supabase CLI via .deb package from GitHub releases
# Note: npm global install is NOT supported by Supabase as of 2025

SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"

EXTENSION_DIR="${HOME}/extensions/supabase-cli"
RESOURCE_DIR="$SCRIPT_DIR/resources"

# Source pkg-manager library if available
if [ -f "${SINDRI_PKG_MANAGER_LIB:-/docker/lib/pkg-manager.sh}" ]; then
    source "${SINDRI_PKG_MANAGER_LIB:-/docker/lib/pkg-manager.sh}"
fi

print_status "Installing Supabase CLI..."

# Check if already installed
if command_exists supabase; then
    current_version=$(supabase --version 2>/dev/null || echo "unknown")
    print_warning "Supabase CLI already installed: $current_version"
    print_status "To upgrade, remove first with: sindri extension remove supabase-cli"
    exit 0
fi

# Detect architecture
ARCH=$(uname -m)
case "$ARCH" in
    x86_64|amd64)
        PKG_ARCH="amd64"
        ;;
    aarch64|arm64)
        PKG_ARCH="arm64"
        ;;
    *)
        print_error "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

# Pinned version for consistency (researched 2026-02-09)
LATEST_VERSION="2.76.15"
print_status "Installing Supabase CLI version: v$LATEST_VERSION"

case "${SINDRI_DISTRO:-ubuntu}" in
  ubuntu)
    # Download .deb package
    DEB_URL="https://github.com/supabase/cli/releases/download/v${LATEST_VERSION}/supabase_${LATEST_VERSION}_linux_${PKG_ARCH}.deb"
    DEB_FILE="/tmp/supabase_${LATEST_VERSION}_linux_${PKG_ARCH}.deb"

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
        cleanup_pkg_cache
        # Retry dpkg
        if ! sudo DEBIAN_FRONTEND=noninteractive dpkg -i "$DEB_FILE"; then
            print_error "Failed to install Supabase CLI .deb package"
            rm -f "$DEB_FILE"
            exit 1
        fi
    fi

    # Clean up deb file and APT caches
    rm -f "$DEB_FILE"
    cleanup_pkg_cache
    ;;

  fedora)
    # Download RPM package
    RPM_URL="https://github.com/supabase/cli/releases/download/v${LATEST_VERSION}/supabase_${LATEST_VERSION}_linux_${PKG_ARCH}.rpm"
    RPM_FILE="/tmp/supabase_${LATEST_VERSION}_linux_${PKG_ARCH}.rpm"

    print_status "Downloading Supabase CLI .rpm package..."
    if ! curl -fsSL -o "$RPM_FILE" "$RPM_URL"; then
        print_error "Failed to download Supabase CLI RPM from: $RPM_URL"
        rm -f "$RPM_FILE"
        exit 1
    fi

    if [[ ! -f "$RPM_FILE" ]] || [[ ! -s "$RPM_FILE" ]]; then
        print_error "Downloaded file is empty or missing"
        rm -f "$RPM_FILE"
        exit 1
    fi

    print_status "Installing .rpm package (requires sudo)..."
    if sudo dnf install -y "$RPM_FILE"; then
        print_success "Supabase CLI installed successfully"
    else
        print_error "Failed to install Supabase CLI .rpm package"
        rm -f "$RPM_FILE"
        exit 1
    fi

    rm -f "$RPM_FILE"
    ;;

  opensuse)
    # Download RPM package (same as Fedora)
    RPM_URL="https://github.com/supabase/cli/releases/download/v${LATEST_VERSION}/supabase_${LATEST_VERSION}_linux_${PKG_ARCH}.rpm"
    RPM_FILE="/tmp/supabase_${LATEST_VERSION}_linux_${PKG_ARCH}.rpm"

    print_status "Downloading Supabase CLI .rpm package..."
    if ! curl -fsSL -o "$RPM_FILE" "$RPM_URL"; then
        print_error "Failed to download Supabase CLI RPM from: $RPM_URL"
        rm -f "$RPM_FILE"
        exit 1
    fi

    if [[ ! -f "$RPM_FILE" ]] || [[ ! -s "$RPM_FILE" ]]; then
        print_error "Downloaded file is empty or missing"
        rm -f "$RPM_FILE"
        exit 1
    fi

    print_status "Installing .rpm package (requires sudo)..."
    if sudo zypper --non-interactive install --allow-unsigned-rpm "$RPM_FILE"; then
        print_success "Supabase CLI installed successfully"
    else
        print_error "Failed to install Supabase CLI .rpm package"
        rm -f "$RPM_FILE"
        exit 1
    fi

    rm -f "$RPM_FILE"
    ;;

  *)
    print_error "Unsupported distro: ${SINDRI_DISTRO:-unknown}"
    exit 1
    ;;
esac

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
