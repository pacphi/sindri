#!/bin/bash
# Install additional infrastructure tools not available via mise or apt
set -euo pipefail

# Use WORKSPACE from environment or derive from HOME
WORKSPACE="${WORKSPACE:-${HOME}/workspace}"

# Install directory
INSTALL_DIR="${WORKSPACE}/bin"
mkdir -p "$INSTALL_DIR"

# Install Pulumi
install_pulumi() {
    print_status "Installing Pulumi..."
    if command -v pulumi > /dev/null 2>&1; then
        print_warning "Pulumi already installed: $(pulumi version)"
        return 0
    fi

    # Pinned version for consistency (updated 2026-02-09)
    # Note: Pulumi installer adds 'v' prefix automatically, so don't include it here
    local PULUMI_VERSION="3.219.0"
    print_status "Installing Pulumi v${PULUMI_VERSION}..."

    # Install specific version
    curl -fsSL https://get.pulumi.com | sh -s -- --version "${PULUMI_VERSION}" || {
        print_warning "Failed to install Pulumi v${PULUMI_VERSION}"
        return 1
    }

    # Add to PATH if not already there
    if [[ -d "$HOME/.pulumi/bin" ]]; then
        export PATH="$HOME/.pulumi/bin:$PATH"
        print_success "Pulumi v${PULUMI_VERSION} installed"
    fi
}

# Install Crossplane CLI
install_crossplane() {
    print_status "Installing Crossplane CLI..."
    if command -v crossplane > /dev/null 2>&1; then
        print_warning "Crossplane already installed"
        return 0
    fi

    # Pinned version for consistency (updated 2026-02-09)
    local CROSSPLANE_VERSION="v2.2.0"
    local OS="linux"
    local ARCH="amd64"

    # Detect ARM64
    if [[ "$(uname -m)" == "aarch64" ]] || [[ "$(uname -m)" == "arm64" ]]; then
        ARCH="arm64"
    fi

    print_status "Installing Crossplane ${CROSSPLANE_VERSION}..."

    # Download specific version from GitHub releases
    local DOWNLOAD_URL="https://releases.crossplane.io/stable/${CROSSPLANE_VERSION}/bin/${OS}_${ARCH}/crank"

    if curl -sL "$DOWNLOAD_URL" -o "$INSTALL_DIR/crossplane"; then
        chmod +x "$INSTALL_DIR/crossplane"
        print_success "Crossplane CLI ${CROSSPLANE_VERSION} installed"
    else
        print_warning "Failed to install Crossplane CLI ${CROSSPLANE_VERSION}"
        return 1
    fi
}

# Install kubectx and kubens
install_kubectx() {
    print_status "Installing kubectx and kubens..."

    # Pinned version for consistency (updated 2026-02-09)
    local KUBECTX_VERSION="v0.9.5"
    print_status "Using kubectx version: $KUBECTX_VERSION"

    local BASE_URL="https://raw.githubusercontent.com/ahmetb/kubectx/${KUBECTX_VERSION}/bin"

    for tool in kubectx kubens; do
        if command -v "$tool" > /dev/null 2>&1; then
            print_warning "$tool already installed"
        else
            curl -sL "${BASE_URL}/${tool}" -o "$INSTALL_DIR/$tool" && \
            chmod +x "$INSTALL_DIR/$tool" && \
            print_success "$tool installed" || \
            print_warning "Failed to install $tool"
        fi
    done
}

# Install Carvel suite
install_carvel() {
    print_status "Installing Carvel suite tools..."

    # Pinned versions for consistency (updated 2026-02-09)
    # Source: https://github.com/carvel-dev/
    declare -A CARVEL_VERSIONS=(
        ["kapp"]="v0.65.0"
        ["ytt"]="v0.52.2"
        ["kbld"]="v0.45.2"
        ["vendir"]="v0.43.0"
        ["imgpkg"]="v0.46.0"
    )

    local CARVEL_TOOLS=(kapp ytt kbld vendir imgpkg)
    local OS="linux"
    local ARCH="amd64"

    # Detect ARM64
    if [[ "$(uname -m)" == "aarch64" ]] || [[ "$(uname -m)" == "arm64" ]]; then
        ARCH="arm64"
    fi

    for tool in "${CARVEL_TOOLS[@]}"; do
        if command -v "$tool" > /dev/null 2>&1; then
            print_warning "$tool already installed"
            continue
        fi

        local VERSION="${CARVEL_VERSIONS[$tool]}"
        print_status "Installing $tool $VERSION..."

        local DOWNLOAD_URL="https://github.com/carvel-dev/${tool}/releases/download/${VERSION}/${tool}-${OS}-${ARCH}"

        curl -sL "$DOWNLOAD_URL" -o "$INSTALL_DIR/$tool" && \
            chmod +x "$INSTALL_DIR/$tool" && \
            print_success "$tool $VERSION installed" || \
            print_warning "Failed to install $tool"
    done
}

# Setup infrastructure workspace
setup_workspace() {
    print_status "Setting up infrastructure workspace..."

    mkdir -p "${WORKSPACE}/infrastructure"/{terraform,ansible,kubernetes,pulumi}
    mkdir -p "$HOME/.kube"

    print_success "Infrastructure workspace created"
}

# Main installation
main() {
    print_status "Installing additional infrastructure tools..."

    # Create install directory
    mkdir -p "$INSTALL_DIR"

    # Install tools
    install_pulumi
    install_crossplane
    install_kubectx
    install_carvel

    # Setup workspace
    setup_workspace

    print_success "Additional infrastructure tools installation complete"

    # Show installed tools (use || true to prevent exit code propagation)
    print_status "Infrastructure tools available:"
    command -v terraform > /dev/null 2>&1 && echo "  ✓ Terraform" || true
    command -v ansible > /dev/null 2>&1 && echo "  ✓ Ansible" || true
    command -v kubectl > /dev/null 2>&1 && echo "  ✓ kubectl" || true
    command -v helm > /dev/null 2>&1 && echo "  ✓ Helm" || true
    command -v k9s > /dev/null 2>&1 && echo "  ✓ k9s" || true
    command -v pulumi > /dev/null 2>&1 && echo "  ✓ Pulumi" || true
    command -v crossplane > /dev/null 2>&1 && echo "  ✓ Crossplane" || true
    command -v kubectx > /dev/null 2>&1 && echo "  ✓ kubectx/kubens" || true
    command -v kapp > /dev/null 2>&1 && echo "  ✓ Carvel suite" || true
}

main "$@"