#!/bin/bash
# k3d-adapter.sh - k3d cluster management for Sindri
#
# This adapter implements the k8s provider interface for k3d
# (K3s in Docker). It is sourced by k8s-adapter.sh.
#
# Supports automatic installation on:
#   - macOS (via Homebrew or official script)
#   - Debian/Ubuntu Linux (via official script)
#
# Required functions:
#   adapter_check_prerequisites - Verify k3d is installed
#   adapter_create              - Create cluster
#   adapter_get_kubeconfig      - Export kubeconfig
#   adapter_destroy             - Delete cluster
#   adapter_exists              - Check if cluster exists
#   adapter_status              - Show cluster info
#   adapter_list                - List all clusters

set -euo pipefail

# Detect operating system
detect_os() {
    local os=""
    case "$(uname -s)" in
        Darwin)
            os="macos"
            ;;
        Linux)
            if [[ -f /etc/debian_version ]] || grep -qi debian /etc/os-release 2>/dev/null || grep -qi ubuntu /etc/os-release 2>/dev/null; then
                os="debian"
            elif [[ -f /etc/os-release ]]; then
                os="linux"
            else
                os="linux"
            fi
            ;;
        *)
            os="unknown"
            ;;
    esac
    echo "$os"
}

# Detect architecture
detect_arch() {
    local arch=""
    case "$(uname -m)" in
        x86_64|amd64)
            arch="amd64"
            ;;
        arm64|aarch64)
            arch="arm64"
            ;;
        *)
            arch="$(uname -m)"
            ;;
    esac
    echo "$arch"
}

# Install k3d on macOS
install_k3d_macos() {
    if command_exists brew; then
        print_status "Installing k3d via Homebrew..."
        brew install k3d
    else
        print_warning "Homebrew not found. Installing k3d via official script..."
        install_k3d_script
    fi
}

# Install k3d on Debian/Ubuntu
install_k3d_debian() {
    install_k3d_script
}

# Install k3d via official installation script
install_k3d_script() {
    print_status "Installing k3d via official script..."

    # The official k3d install script handles OS and arch detection
    if curl -s https://raw.githubusercontent.com/k3d-io/k3d/main/install.sh | bash; then
        print_success "k3d installed successfully"
    else
        print_error "Failed to install k3d via script"
        return 1
    fi
}

# Install k3d via direct binary download (alternative method)
install_k3d_binary() {
    local os arch k3d_url

    case "$(uname -s)" in
        Darwin) os="darwin" ;;
        Linux)  os="linux" ;;
        *)      print_error "Unsupported OS: $(uname -s)"; return 1 ;;
    esac

    arch=$(detect_arch)

    # Get latest version
    local version
    version=$(curl -sL https://api.github.com/repos/k3d-io/k3d/releases/latest | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

    if [[ -z "$version" ]]; then
        version="v5.7.4"  # Fallback version
        print_warning "Could not determine latest version, using $version"
    fi

    k3d_url="https://github.com/k3d-io/k3d/releases/download/${version}/k3d-${os}-${arch}"

    print_status "Downloading k3d ${version} for ${os}/${arch}..."

    local install_dir="/usr/local/bin"
    local use_sudo=""

    # Check if we need sudo
    if [[ ! -w "$install_dir" ]]; then
        use_sudo="sudo"
        print_status "Installation requires sudo access..."
    fi

    if curl -Lo /tmp/k3d "$k3d_url"; then
        chmod +x /tmp/k3d
        $use_sudo mv /tmp/k3d "$install_dir/k3d"
        print_success "k3d installed to $install_dir/k3d"
    else
        print_error "Failed to download k3d"
        return 1
    fi
}

# Prompt user to install k3d
prompt_install_k3d() {
    local os
    os=$(detect_os)

    echo ""
    print_warning "k3d is not installed"
    echo ""

    case "$os" in
        macos)
            echo "  k3d can be installed via:"
            echo "    • Homebrew: brew install k3d"
            echo "    • Official script: curl -s https://raw.githubusercontent.com/k3d-io/k3d/main/install.sh | bash"
            ;;
        debian|linux)
            echo "  k3d can be installed via:"
            echo "    • Official script: curl -s https://raw.githubusercontent.com/k3d-io/k3d/main/install.sh | bash"
            echo "    • Direct download from: https://github.com/k3d-io/k3d/releases"
            ;;
        *)
            echo "  Install from: https://k3d.io/#installation"
            return 1
            ;;
    esac

    echo ""
    read -p "Would you like to install k3d now? (y/N) " -n 1 -r
    echo

    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        print_status "Skipping installation"
        return 1
    fi

    case "$os" in
        macos)
            install_k3d_macos
            ;;
        debian|linux)
            install_k3d_debian
            ;;
    esac

    # Verify installation
    if command_exists k3d; then
        print_success "k3d installed successfully: $(k3d version)"
        return 0
    else
        print_error "k3d installation failed"
        return 1
    fi
}

# Check if k3d and docker are available
adapter_check_prerequisites() {
    local errors=0

    # Check for k3d, offer to install if missing
    if ! command_exists k3d; then
        if ! prompt_install_k3d; then
            errors=$((errors + 1))
        fi
    fi

    # Check for Docker
    if ! command_exists docker; then
        print_error "Docker is required for k3d"
        local os
        os=$(detect_os)
        case "$os" in
            macos)
                print_status "  Install Docker Desktop: https://www.docker.com/products/docker-desktop/"
                ;;
            debian)
                print_status "  Install Docker: sudo apt-get install docker.io docker-compose-v2"
                print_status "  Or Docker Desktop: https://www.docker.com/products/docker-desktop/"
                ;;
        esac
        errors=$((errors + 1))
    elif ! docker info >/dev/null 2>&1; then
        print_error "Docker daemon is not running"
        print_status "  Please start Docker and try again"
        errors=$((errors + 1))
    fi

    return $errors
}

# Create a k3d cluster
# Globals: CLUSTER_NAME, K8S_VERSION, NODES, K3D_IMAGE, K3D_REGISTRY, K3D_REGISTRY_NAME, K3D_REGISTRY_PORT
adapter_create() {
    local cluster_name="${CLUSTER_NAME:-sindri-local}"

    adapter_check_prerequisites || return 1

    if adapter_exists "$cluster_name"; then
        print_warning "Cluster '$cluster_name' already exists"
        print_status "Context: k3d-${cluster_name}"
        return 0
    fi

    print_header "Creating k3d cluster: $cluster_name"

    local k3d_args=("cluster" "create" "$cluster_name")

    # Use custom image if specified
    if [[ -n "${K3D_IMAGE:-}" ]]; then
        k3d_args+=("--image" "$K3D_IMAGE")
    fi

    # Add worker nodes if multi-node requested
    if [[ "${NODES:-1}" -gt 1 ]]; then
        local agents=$((NODES - 1))
        k3d_args+=("--agents" "$agents")
        print_status "Creating cluster with 1 server and ${agents} agent(s)"
    fi

    # Registry support
    if [[ "${K3D_REGISTRY:-false}" == "true" ]]; then
        local registry_name="${K3D_REGISTRY_NAME:-k3d-registry}"
        local registry_port="${K3D_REGISTRY_PORT:-5000}"
        k3d_args+=("--registry-create" "${registry_name}:0.0.0.0:${registry_port}")
        print_status "Creating local registry: ${registry_name}:${registry_port}"
    fi

    k3d "${k3d_args[@]}"

    print_success "Cluster '$cluster_name' created"
    print_status "Context: k3d-${cluster_name}"

    # Show registry info if created
    if [[ "${K3D_REGISTRY:-false}" == "true" ]]; then
        echo ""
        print_status "Local registry available at: localhost:${K3D_REGISTRY_PORT:-5000}"
        print_status "Push images: docker tag myimage localhost:${K3D_REGISTRY_PORT:-5000}/myimage && docker push localhost:${K3D_REGISTRY_PORT:-5000}/myimage"
    fi

    echo ""
    print_status "To use with DevPod, add to sindri.yaml:"
    echo "  providers:"
    echo "    devpod:"
    echo "      type: kubernetes"
    echo "      kubernetes:"
    echo "        context: k3d-${cluster_name}"
}

# Get kubeconfig for the cluster
adapter_get_kubeconfig() {
    local cluster_name="${1:-${CLUSTER_NAME:-sindri-local}}"

    if ! adapter_exists "$cluster_name"; then
        print_error "Cluster '$cluster_name' does not exist"
        return 1
    fi

    k3d kubeconfig get "$cluster_name"
}

# Destroy the cluster
adapter_destroy() {
    local cluster_name="${1:-${CLUSTER_NAME:-sindri-local}}"
    local force="${2:-}"

    if ! adapter_exists "$cluster_name"; then
        print_warning "Cluster '$cluster_name' does not exist"
        return 0
    fi

    # Confirmation if not forced
    if [[ "$force" != "--force" ]] && [[ "$force" != "-f" ]]; then
        print_warning "This will destroy cluster: $cluster_name"
        read -p "Are you sure? (y/N) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            print_status "Cancelled"
            return 0
        fi
    fi

    print_header "Deleting k3d cluster: $cluster_name"
    k3d cluster delete "$cluster_name"
    print_success "Cluster '$cluster_name' deleted"
}

# Check if cluster exists
adapter_exists() {
    local cluster_name="${1:-${CLUSTER_NAME:-sindri-local}}"

    # k3d cluster list returns JSON, check for cluster name
    if command_exists jq; then
        k3d cluster list -o json 2>/dev/null | jq -e ".[] | select(.name == \"$cluster_name\")" >/dev/null 2>&1
    else
        # Fallback to grep on text output
        k3d cluster list 2>/dev/null | grep -q "^${cluster_name}\s"
    fi
}

# Show cluster status
adapter_status() {
    local cluster_name="${1:-${CLUSTER_NAME:-sindri-local}}"

    if ! adapter_exists "$cluster_name"; then
        echo "Cluster: $cluster_name"
        echo "Status: Not found"
        return 1
    fi

    local context="k3d-${cluster_name}"

    echo "Cluster: $cluster_name"
    echo "Provider: k3d"
    echo "Context: $context"
    echo ""

    # Show k3d cluster info
    print_status "Cluster details:"
    k3d cluster list 2>/dev/null | grep -E "^(NAME|${cluster_name})" || true
    echo ""

    # Check if we can connect
    if kubectl --context "$context" cluster-info >/dev/null 2>&1; then
        echo "Status: Running"
        echo ""
        echo "Nodes:"
        kubectl --context "$context" get nodes -o wide 2>/dev/null || true
    else
        echo "Status: Not accessible (Docker may be stopped)"
    fi
}

# List all k3d clusters
adapter_list() {
    local clusters

    if command_exists jq; then
        clusters=$(k3d cluster list -o json 2>/dev/null | jq -r '.[].name' || true)
    else
        clusters=$(k3d cluster list 2>/dev/null | tail -n +2 | awk '{print $1}' || true)
    fi

    if [[ -z "$clusters" ]]; then
        return 0
    fi

    echo "$clusters" | while read -r cluster; do
        [[ -n "$cluster" ]] && echo "  - $cluster (context: k3d-${cluster})"
    done
}

# Export functions
export -f detect_os detect_arch install_k3d_macos install_k3d_debian install_k3d_script install_k3d_binary prompt_install_k3d
export -f adapter_check_prerequisites adapter_create adapter_get_kubeconfig
export -f adapter_destroy adapter_exists adapter_status adapter_list
