#!/bin/bash
# Install additional infrastructure tools not available via mise or apt
set -euo pipefail

# Source common utilities
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$(dirname "$(dirname "$SCRIPT_DIR")")/common.sh"

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

    curl -fsSL https://get.pulumi.com | sh || {
        print_warning "Failed to install Pulumi"
        return 1
    }

    # Add to PATH if not already there
    if [[ -d "$HOME/.pulumi/bin" ]]; then
        export PATH="$HOME/.pulumi/bin:$PATH"
        print_success "Pulumi installed"
    fi
}

# Install Crossplane CLI
install_crossplane() {
    print_status "Installing Crossplane CLI..."
    if command -v crossplane > /dev/null 2>&1; then
        print_warning "Crossplane already installed"
        return 0
    fi

    curl -sL https://raw.githubusercontent.com/crossplane/crossplane/master/install.sh | sh || {
        print_warning "Failed to install Crossplane CLI"
        return 1
    }

    if [[ -f "./kubectl-crossplane" ]]; then
        chmod +x ./kubectl-crossplane
        mv ./kubectl-crossplane "$INSTALL_DIR/crossplane"
        print_success "Crossplane CLI installed"
    fi
}

# Install kubectx and kubens
install_kubectx() {
    print_status "Installing kubectx and kubens..."

    local KUBECTX_VERSION="v0.9.5"
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

    local CARVEL_TOOLS=(kapp ytt kbld vendir imgpkg)

    for tool in "${CARVEL_TOOLS[@]}"; do
        if command -v "$tool" > /dev/null 2>&1; then
            print_warning "$tool already installed"
            continue
        fi

        print_status "Installing $tool..."

        case "$tool" in
            kapp)
                curl -sL https://carvel.dev/install.sh | K14SIO_INSTALL_BIN_DIR="$INSTALL_DIR" bash -s -- kapp
                ;;
            ytt)
                curl -sL https://carvel.dev/install.sh | K14SIO_INSTALL_BIN_DIR="$INSTALL_DIR" bash -s -- ytt
                ;;
            kbld)
                curl -sL https://carvel.dev/install.sh | K14SIO_INSTALL_BIN_DIR="$INSTALL_DIR" bash -s -- kbld
                ;;
            vendir)
                curl -sL https://carvel.dev/install.sh | K14SIO_INSTALL_BIN_DIR="$INSTALL_DIR" bash -s -- vendir
                ;;
            imgpkg)
                curl -sL https://carvel.dev/install.sh | K14SIO_INSTALL_BIN_DIR="$INSTALL_DIR" bash -s -- imgpkg
                ;;
        esac

        if [[ -f "$INSTALL_DIR/$tool" ]]; then
            chmod +x "$INSTALL_DIR/$tool"
            print_success "$tool installed"
        else
            print_warning "Failed to install $tool"
        fi
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

    # Show installed tools
    print_status "Infrastructure tools available:"
    command -v terraform > /dev/null 2>&1 && echo "  ✓ Terraform"
    command -v ansible > /dev/null 2>&1 && echo "  ✓ Ansible"
    command -v kubectl > /dev/null 2>&1 && echo "  ✓ kubectl"
    command -v helm > /dev/null 2>&1 && echo "  ✓ Helm"
    command -v k9s > /dev/null 2>&1 && echo "  ✓ k9s"
    command -v pulumi > /dev/null 2>&1 && echo "  ✓ Pulumi"
    command -v crossplane > /dev/null 2>&1 && echo "  ✓ Crossplane"
    command -v kubectx > /dev/null 2>&1 && echo "  ✓ kubectx/kubens"
    command -v kapp > /dev/null 2>&1 && echo "  ✓ Carvel suite"
}

main "$@"