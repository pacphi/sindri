#!/usr/bin/env bash
# =============================================================================
# Sysbox Host Setup Script
# =============================================================================
#
# This script installs Sysbox on the HOST machine (not inside Sindri container)
# to enable secure Docker-in-Docker support without privileged mode.
#
# Uses gh CLI for GitHub API access to:
#   - Fetch latest Sysbox release version dynamically
#   - Avoid rate limiting issues
#   - Keep script up-to-date as new versions are released
#
# Usage:
#   ./scripts/setup-sysbox-host.sh
#   ./scripts/setup-sysbox-host.sh --version v0.6.7  # Install specific version
#
# Requirements:
#   - Ubuntu 18.04-24.04 or Debian 10-11
#   - Docker installed (not via snap)
#   - Linux kernel 5.12+ recommended (5.19+ optimal)
#   - Root/sudo access
#   - gh CLI installed (optional, but recommended for rate limits)
#
# After installation, Sindri containers can use DinD securely:
#   providers:
#     docker:
#       dind:
#         enabled: true
#         mode: sysbox  # or auto (will detect sysbox)
#
# =============================================================================

set -euo pipefail

# GitHub repository
SYSBOX_REPO="nestybox/sysbox"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Output functions
print_header() {
    echo -e "\n${BLUE}=== $1 ===${NC}\n"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_info() {
    echo -e "  $1"
}

print_debug() {
    if [[ "${DEBUG:-false}" == "true" ]]; then
        echo -e "${BLUE}[DEBUG]${NC} $1"
    fi
}

# =============================================================================
# GitHub Release Version Detection (mirrors common.sh pattern)
# =============================================================================

# Get latest GitHub release version
# Uses gh CLI as primary method with curl fallback for reliability
get_github_release_version() {
    local repo="$1"
    local include_v="${2:-true}"
    local version=""

    # Method 1: gh CLI (handles auth automatically, avoids rate limits)
    if command -v gh &>/dev/null; then
        print_debug "Fetching version for $repo via gh CLI..."
        # Sysbox uses stable releases only
        version=$(gh release view --repo "$repo" --json tagName --jq '.tagName' 2>/dev/null || echo "")
        if [[ -n "$version" ]]; then
            print_debug "Got version $version via gh CLI"
        fi
    fi

    # Method 2: curl with GitHub API (fallback)
    if [[ -z "$version" ]]; then
        print_debug "Fetching version for $repo via GitHub API..."
        local curl_args=(-fsSL)
        if [[ -n "${GITHUB_TOKEN:-}" ]]; then
            curl_args+=(-H "Authorization: Bearer ${GITHUB_TOKEN}")
        fi
        version=$(curl "${curl_args[@]}" "https://api.github.com/repos/${repo}/releases/latest" 2>/dev/null | \
            grep '"tag_name":' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/' || echo "")
        if [[ -n "$version" ]]; then
            print_debug "Got version $version via GitHub API"
        fi
    fi

    # Process version string
    if [[ -n "$version" ]]; then
        if [[ "$include_v" == "false" ]]; then
            version="${version#v}"
        fi
        echo "$version"
    fi
}

# Download GitHub release asset
download_github_release_asset() {
    local repo="$1"
    local version="$2"
    local asset_pattern="$3"
    local output_path="$4"

    # Method 1: gh CLI (handles auth, avoids rate limits)
    if command -v gh &>/dev/null; then
        print_debug "Downloading via gh CLI: $asset_pattern"
        if gh release download "$version" --repo "$repo" --pattern "$asset_pattern" --output "$output_path" 2>/dev/null; then
            return 0
        fi
    fi

    # Method 2: Direct URL (fallback)
    print_debug "Downloading via direct URL..."
    local download_url="https://github.com/${repo}/releases/download/${version}/${asset_pattern}"
    local curl_args=(-fsSL -o "$output_path")
    if [[ -n "${GITHUB_TOKEN:-}" ]]; then
        curl_args+=(-H "Authorization: Bearer ${GITHUB_TOKEN}")
    fi
    curl "${curl_args[@]}" "$download_url"
}

# =============================================================================
# Prerequisites Check
# =============================================================================

check_prerequisites() {
    print_header "Checking Prerequisites"

    # Check if running as root or with sudo
    if [[ $EUID -ne 0 ]]; then
        if ! command -v sudo &>/dev/null; then
            print_error "This script requires root privileges. Please run with sudo."
            exit 1
        fi
    fi

    # Check OS
    if [[ ! -f /etc/os-release ]]; then
        print_error "Cannot determine OS. Sysbox requires Ubuntu or Debian."
        exit 1
    fi

    # shellcheck source=/dev/null
    source /etc/os-release

    case "$ID" in
        ubuntu)
            print_success "OS: $PRETTY_NAME"
            case "$VERSION_CODENAME" in
                bionic|focal|jammy|noble)
                    print_info "Distribution version supported"
                    ;;
                *)
                    print_warning "Version $VERSION_CODENAME may not be fully tested"
                    ;;
            esac
            ;;
        debian)
            print_success "OS: $PRETTY_NAME"
            case "$VERSION_CODENAME" in
                buster|bullseye|bookworm)
                    print_info "Distribution version supported"
                    ;;
                *)
                    print_warning "Version $VERSION_CODENAME may not be fully tested"
                    ;;
            esac
            ;;
        *)
            print_error "Unsupported OS: $ID"
            print_info "Sysbox packages are available for Ubuntu/Debian only."
            print_info "For other distros, build from source:"
            print_info "  https://github.com/nestybox/sysbox/blob/master/docs/user-guide/install-source.md"
            exit 1
            ;;
    esac

    # Check architecture
    ARCH=$(uname -m)
    case "$ARCH" in
        x86_64)
            DEB_ARCH="amd64"
            print_success "Architecture: amd64"
            ;;
        aarch64)
            DEB_ARCH="arm64"
            print_success "Architecture: arm64"
            ;;
        *)
            print_error "Unsupported architecture: $ARCH"
            print_info "Sysbox supports amd64 and arm64 only"
            exit 1
            ;;
    esac

    # Check kernel version
    local kernel_version
    kernel_version=$(uname -r | cut -d. -f1-2)
    local kernel_major
    kernel_major=$(echo "$kernel_version" | cut -d. -f1)
    local kernel_minor
    kernel_minor=$(echo "$kernel_version" | cut -d. -f2)

    if [[ "$kernel_major" -lt 5 ]]; then
        print_error "Kernel $kernel_version is too old. Minimum 5.4 required."
        exit 1
    elif [[ "$kernel_major" -eq 5 && "$kernel_minor" -lt 4 ]]; then
        print_error "Kernel $kernel_version is too old. Minimum 5.4 required."
        exit 1
    elif [[ "$kernel_major" -eq 5 && "$kernel_minor" -lt 12 ]]; then
        print_warning "Kernel: $kernel_version (shiftfs may be required)"
        print_info "Consider upgrading to kernel 5.12+ for better compatibility"
    elif [[ "$kernel_major" -eq 5 && "$kernel_minor" -lt 19 ]]; then
        print_success "Kernel: $kernel_version (shiftfs recommended but not required)"
    else
        print_success "Kernel: $kernel_version (optimal)"
    fi

    # Check Docker installation
    if ! command -v docker &>/dev/null; then
        print_error "Docker not installed"
        print_info "Install Docker first: https://docs.docker.com/engine/install/"
        exit 1
    fi

    local docker_version
    docker_version=$(docker --version | cut -d' ' -f3 | tr -d ',')
    print_success "Docker: $docker_version"

    # Check if Docker was installed via snap (not supported)
    if command -v snap &>/dev/null && snap list docker &>/dev/null 2>&1; then
        print_error "Docker installed via snap (not supported by Sysbox)"
        print_info "Remove snap Docker and install from official repository:"
        print_info "  sudo snap remove docker"
        print_info "  https://docs.docker.com/engine/install/"
        exit 1
    fi

    # Check systemd
    if ! command -v systemctl &>/dev/null; then
        print_error "systemd not found"
        print_info "Sysbox requires systemd as the process manager"
        exit 1
    fi
    print_success "systemd: available"

    # Check gh CLI (optional but recommended)
    if command -v gh &>/dev/null; then
        print_success "gh CLI: installed (recommended for avoiding rate limits)"
    else
        print_warning "gh CLI: not installed"
        print_info "Install gh CLI for better reliability: https://cli.github.com/"
        print_info "Falling back to direct GitHub API (may hit rate limits)"
    fi

    # Check/install jq dependency
    if ! command -v jq &>/dev/null; then
        print_info "Installing jq dependency..."
        sudo apt-get update -qq
        sudo apt-get install -y -qq jq
    fi
    print_success "jq: installed"

    echo ""
}

# =============================================================================
# Installation
# =============================================================================

install_sysbox() {
    print_header "Installing Sysbox"

    # Determine version to install
    local version="$REQUESTED_VERSION"
    if [[ -z "$version" || "$version" == "latest" ]]; then
        print_info "Fetching latest Sysbox release version..."
        version=$(get_github_release_version "$SYSBOX_REPO" true)
        if [[ -z "$version" ]]; then
            print_error "Failed to fetch latest version from GitHub"
            print_info "Check network connectivity and GitHub access"
            print_info "Try installing gh CLI for better reliability: https://cli.github.com/"
            exit 1
        fi
    fi

    # Ensure version has v prefix
    [[ "$version" != v* ]] && version="v$version"

    print_success "Installing Sysbox $version"

    # Construct package filename
    # Sysbox uses format: sysbox-ce_0.6.7-0.linux_amd64.deb
    local version_num="${version#v}"
    local deb_filename="sysbox-ce_${version_num}-0.linux_${DEB_ARCH}.deb"
    local download_path="/tmp/${deb_filename}"

    # Download package
    print_info "Downloading ${deb_filename}..."
    if ! download_github_release_asset "$SYSBOX_REPO" "$version" "$deb_filename" "$download_path"; then
        print_error "Failed to download Sysbox package"
        print_info "Check if version $version exists: https://github.com/$SYSBOX_REPO/releases"
        exit 1
    fi

    # Verify download
    if [[ ! -s "$download_path" ]]; then
        print_error "Download failed or file is empty"
        rm -f "$download_path"
        exit 1
    fi
    print_success "Download complete"

    # Stop and remove existing containers (required by Sysbox installer)
    local running_containers
    running_containers=$(docker ps -q 2>/dev/null || true)
    if [[ -n "$running_containers" ]]; then
        print_warning "Stopping running containers..."
        # shellcheck disable=SC2086
        docker stop $running_containers || true
    fi

    local all_containers
    all_containers=$(docker ps -aq 2>/dev/null || true)
    if [[ -n "$all_containers" ]]; then
        print_warning "Removing containers (required for Sysbox installation)..."
        # shellcheck disable=SC2086
        docker rm $all_containers || true
    fi

    # Install package
    print_info "Installing Sysbox package..."
    if sudo apt-get install -y "$download_path"; then
        print_success "Sysbox $version installed"
    else
        print_error "Installation failed"
        rm -f "$download_path"
        exit 1
    fi

    # Clean up
    rm -f "$download_path"

    echo ""
}

# =============================================================================
# Verification
# =============================================================================

verify_installation() {
    print_header "Verifying Installation"

    # Check systemd services
    if sudo systemctl is-active --quiet sysbox; then
        print_success "sysbox.service: active"
    else
        print_error "sysbox.service: not active"
        print_info "Checking service status..."
        sudo systemctl status sysbox --no-pager || true
        exit 1
    fi

    # Check individual components
    for service in sysbox-mgr sysbox-fs; do
        if sudo systemctl is-active --quiet "$service"; then
            print_success "${service}.service: active"
        else
            print_warning "${service}.service: not active"
        fi
    done

    # Check Docker recognizes runtime
    if docker info 2>/dev/null | grep -q "sysbox-runc"; then
        print_success "Docker runtime: sysbox-runc registered"
    else
        print_error "Docker does not recognize sysbox-runc"
        print_info "Check /etc/docker/daemon.json for runtime configuration"
        print_info "Try: sudo systemctl restart docker"
        exit 1
    fi

    # Test run
    print_info "Testing Sysbox container..."
    if docker run --rm --runtime=sysbox-runc alpine echo "Sysbox test successful" 2>/dev/null; then
        print_success "Test container: passed"
    else
        print_error "Test container: failed"
        print_info "Check sysbox logs: journalctl -u sysbox -n 50"
        exit 1
    fi

    echo ""
}

# =============================================================================
# Summary
# =============================================================================

print_summary() {
    print_header "Sysbox Setup Complete"

    # Get installed version
    local installed_version
    installed_version=$(dpkg -l sysbox-ce 2>/dev/null | grep sysbox-ce | awk '{print $3}' | cut -d'-' -f1 || echo "unknown")

    echo "Sysbox v${installed_version} is now installed and running."
    echo ""
    echo "Sindri containers can now use Docker-in-Docker securely:"
    echo ""
    echo "  # sindri.yaml"
    echo "  providers:"
    echo "    docker:"
    echo "      dind:"
    echo "        enabled: true"
    echo "        mode: sysbox  # Explicit"
    echo ""
    echo "Or use auto-detection (recommended):"
    echo ""
    echo "  # sindri.yaml"
    echo "  providers:"
    echo "    docker:"
    echo "      dind:"
    echo "        enabled: true"
    echo "        mode: auto  # Will detect sysbox"
    echo ""
    echo "Benefits of Sysbox DinD:"
    echo "  - No privileged mode required"
    echo "  - Native overlay2 storage driver"
    echo "  - Full systemd support inside containers"
    echo "  - User-namespace isolation (root → unprivileged)"
    echo ""
    echo "Documentation:"
    echo "  - Sysbox: https://github.com/nestybox/sysbox"
    echo "  - Sindri Docker: docs/extensions/DOCKER.md"
    echo ""
}

# =============================================================================
# Main
# =============================================================================

usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Install Sysbox on the host machine for secure Docker-in-Docker support."
    echo ""
    echo "Options:"
    echo "  --version VERSION    Install specific version (e.g., v0.6.7)"
    echo "  --debug              Enable debug output"
    echo "  --help, -h           Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0                   # Install latest version"
    echo "  $0 --version v0.6.7  # Install specific version"
    echo ""
}

# Parse arguments
REQUESTED_VERSION=""
DEB_ARCH=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --version)
            REQUESTED_VERSION="$2"
            shift 2
            ;;
        --debug)
            DEBUG=true
            shift
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

main() {
    echo ""
    echo "╔══════════════════════════════════════════════════════════════════╗"
    echo "║           Sysbox Host Setup for Sindri DinD Support              ║"
    echo "╚══════════════════════════════════════════════════════════════════╝"
    echo ""

    check_prerequisites
    install_sysbox
    verify_installation
    print_summary
}

# Run main
main
