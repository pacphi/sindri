#!/bin/bash
set -euo pipefail

print_status "Installing sindri-agent..."

GITHUB_REPO="pacphi/sindri"
BINARY_NAME="sindri-agent"
INSTALL_PATH="$HOME/.local/bin"

# Check if already installed
if [[ -f "${INSTALL_PATH}/${BINARY_NAME}" ]]; then
    if "${INSTALL_PATH}/${BINARY_NAME}" version >/dev/null 2>&1; then
        current_version=$("${INSTALL_PATH}/${BINARY_NAME}" version 2>/dev/null | head -n1 || echo "unknown")
        print_warning "sindri-agent is already installed: $current_version"
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

# Get latest release version
print_status "Fetching latest release..."
tag_name=$(get_github_release_version "${GITHUB_REPO}" true false)

if [[ -z "$tag_name" ]]; then
    print_warning "Could not fetch latest release, using fallback console-agent-v1.0.0"
    tag_name="console-agent-v1.0.0"
fi

# Strip leading 'v' if present and build full tag
version="${tag_name#v}"
print_status "Latest release: ${tag_name}"

# Detect platform and architecture
case "$(uname -s)" in
    Linux)  goos="linux"  ;;
    Darwin) goos="darwin" ;;
    *)
        print_error "Unsupported OS: $(uname -s)"
        exit 1
        ;;
esac

case "$(uname -m)" in
    x86_64|amd64)   goarch="amd64" ;;
    aarch64|arm64)  goarch="arm64" ;;
    *)
        print_error "Unsupported architecture: $(uname -m)"
        exit 1
        ;;
esac

print_status "Detected platform: ${goos}/${goarch}"

# Build download URL
# Binary naming convention: sindri-agent-<goos>-<goarch>
# e.g., sindri-agent-linux-amd64, sindri-agent-darwin-arm64
release_binary="${BINARY_NAME}-${goos}-${goarch}"
download_url="https://github.com/${GITHUB_REPO}/releases/download/${tag_name}/${release_binary}"

print_status "Downloading sindri-agent from ${download_url}..."
mkdir -p "${INSTALL_PATH}"

if curl -fsSL -o "${INSTALL_PATH}/${BINARY_NAME}" "${download_url}"; then
    print_success "Binary downloaded successfully"
else
    print_error "Failed to download binary from ${download_url}"
    exit 1
fi

# Make executable
chmod +x "${INSTALL_PATH}/${BINARY_NAME}"

# Verify the binary works
if "${INSTALL_PATH}/${BINARY_NAME}" --version >/dev/null 2>&1 || \
   "${INSTALL_PATH}/${BINARY_NAME}" version >/dev/null 2>&1; then
    print_success "sindri-agent installed successfully"
else
    print_warning "Binary downloaded but version check failed - binary may still work at runtime"
fi

# Create config directory
mkdir -p "${HOME}/.config/sindri-agent"
print_success "Created config directory at ~/.config/sindri-agent"

print_success "sindri-agent installation complete"
print_status "Run configure-agent.sh to configure the agent, then start-agent.sh to start it"
