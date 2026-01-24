#!/bin/bash
# kind-adapter.sh - kind cluster management for Sindri
#
# This adapter implements the k8s provider interface for kind
# (Kubernetes IN Docker). It is sourced by k8s-adapter.sh.
#
# Supports automatic installation on:
#   - macOS (via Homebrew)
#   - Debian/Ubuntu Linux (via apt or direct download)
#
# Required functions:
#   adapter_check_prerequisites - Verify kind is installed
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

# Install kind on macOS
install_kind_macos() {
    if command_exists brew; then
        print_status "Installing kind via Homebrew..."
        brew install kind
    else
        print_warning "Homebrew not found. Installing kind via direct download..."
        install_kind_binary
    fi
}

# Install kind on Debian/Ubuntu
install_kind_debian() {
    local arch
    arch=$(detect_arch)

    # Try go install first if Go is available
    if command_exists go; then
        print_status "Installing kind via go install..."
        go install sigs.k8s.io/kind@latest
        # Add GOPATH/bin to PATH if needed
        if [[ -d "$HOME/go/bin" ]] && [[ ":$PATH:" != *":$HOME/go/bin:"* ]]; then
            export PATH="$HOME/go/bin:$PATH"
        fi
        return 0
    fi

    # Fall back to binary download
    install_kind_binary
}

# Install kind via direct binary download (works on any platform)
install_kind_binary() {
    local os arch kind_url

    case "$(uname -s)" in
        Darwin) os="darwin" ;;
        Linux)  os="linux" ;;
        *)      print_error "Unsupported OS: $(uname -s)"; return 1 ;;
    esac

    arch=$(detect_arch)

    # Get latest version
    local version
    version=$(curl -sL https://api.github.com/repos/kubernetes-sigs/kind/releases/latest | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

    if [[ -z "$version" ]]; then
        version="v0.25.0"  # Fallback version
        print_warning "Could not determine latest version, using $version"
    fi

    kind_url="https://kind.sigs.k8s.io/dl/${version}/kind-${os}-${arch}"

    print_status "Downloading kind ${version} for ${os}/${arch}..."

    local install_dir="/usr/local/bin"
    local use_sudo=""

    # Check if we need sudo
    if [[ ! -w "$install_dir" ]]; then
        use_sudo="sudo"
        print_status "Installation requires sudo access..."
    fi

    if curl -Lo /tmp/kind "$kind_url"; then
        chmod +x /tmp/kind
        $use_sudo mv /tmp/kind "$install_dir/kind"
        print_success "kind installed to $install_dir/kind"
    else
        print_error "Failed to download kind"
        return 1
    fi
}

# Prompt user to install kind
prompt_install_kind() {
    local os
    os=$(detect_os)

    echo ""
    print_warning "kind is not installed"
    echo ""

    case "$os" in
        macos)
            echo "  kind can be installed via:"
            echo "    • Homebrew: brew install kind"
            echo "    • Direct download: curl -Lo ./kind https://kind.sigs.k8s.io/dl/latest/kind-darwin-$(detect_arch)"
            ;;
        debian)
            echo "  kind can be installed via:"
            echo "    • Go: go install sigs.k8s.io/kind@latest"
            echo "    • Direct download: curl -Lo ./kind https://kind.sigs.k8s.io/dl/latest/kind-linux-$(detect_arch)"
            ;;
        *)
            echo "  Install from: https://kind.sigs.k8s.io/docs/user/quick-start/#installation"
            return 1
            ;;
    esac

    echo ""
    read -p "Would you like to install kind now? (y/N) " -n 1 -r
    echo

    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        print_status "Skipping installation"
        return 1
    fi

    case "$os" in
        macos)
            install_kind_macos
            ;;
        debian|linux)
            install_kind_debian
            ;;
    esac

    # Verify installation
    if command_exists kind; then
        print_success "kind installed successfully: $(kind version)"
        return 0
    else
        print_error "kind installation failed"
        return 1
    fi
}

# Check if kind and docker are available
adapter_check_prerequisites() {
    local errors=0

    # Check for kind, offer to install if missing
    if ! command_exists kind; then
        if ! prompt_install_kind; then
            errors=$((errors + 1))
        fi
    fi

    # Check for Docker
    if ! command_exists docker; then
        print_error "Docker is required for kind"
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

# Create a kind cluster
# Globals: CLUSTER_NAME, K8S_VERSION, NODES, KIND_IMAGE, KIND_CONFIG
adapter_create() {
    local cluster_name="${CLUSTER_NAME:-sindri-local}"

    adapter_check_prerequisites || return 1

    if adapter_exists "$cluster_name"; then
        print_warning "Cluster '$cluster_name' already exists"
        print_status "Context: kind-${cluster_name}"
        return 0
    fi

    print_header "Creating kind cluster: $cluster_name"

    local kind_args=("create" "cluster" "--name" "$cluster_name")

    # Use custom image if specified
    if [[ -n "${KIND_IMAGE:-}" ]]; then
        kind_args+=("--image" "$KIND_IMAGE")
    fi

    # Use custom config file if specified
    if [[ -n "${KIND_CONFIG:-}" ]] && [[ -f "$KIND_CONFIG" ]]; then
        kind_args+=("--config" "$KIND_CONFIG")
        kind "${kind_args[@]}"
    elif [[ "${NODES:-1}" -gt 1 ]]; then
        # Generate multi-node configuration
        print_status "Creating multi-node cluster with ${NODES} nodes"
        local config
        config=$(cat << EOF
kind: Cluster
apiVersion: kind.x-k8s.io/v1alpha4
nodes:
- role: control-plane
$(for _ in $(seq 2 "${NODES}"); do echo "- role: worker"; done)
EOF
)
        echo "$config" | kind create cluster --name "$cluster_name" --config -
    else
        # Single node cluster
        kind "${kind_args[@]}"
    fi

    print_success "Cluster '$cluster_name' created"
    print_status "Context: kind-${cluster_name}"
    echo ""
    print_status "To use with DevPod, add to sindri.yaml:"
    echo "  providers:"
    echo "    devpod:"
    echo "      type: kubernetes"
    echo "      kubernetes:"
    echo "        context: kind-${cluster_name}"
}

# Get kubeconfig for the cluster
adapter_get_kubeconfig() {
    local cluster_name="${1:-${CLUSTER_NAME:-sindri-local}}"

    if ! adapter_exists "$cluster_name"; then
        print_error "Cluster '$cluster_name' does not exist"
        return 1
    fi

    kind get kubeconfig --name "$cluster_name"
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

    print_header "Deleting kind cluster: $cluster_name"
    kind delete cluster --name "$cluster_name"
    print_success "Cluster '$cluster_name' deleted"
}

# Check if cluster exists
adapter_exists() {
    local cluster_name="${1:-${CLUSTER_NAME:-sindri-local}}"
    kind get clusters 2>/dev/null | grep -q "^${cluster_name}$"
}

# Show cluster status
adapter_status() {
    local cluster_name="${1:-${CLUSTER_NAME:-sindri-local}}"

    if ! adapter_exists "$cluster_name"; then
        echo "Cluster: $cluster_name"
        echo "Status: Not found"
        return 1
    fi

    local context="kind-${cluster_name}"

    echo "Cluster: $cluster_name"
    echo "Provider: kind"
    echo "Context: $context"
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

# List all kind clusters
adapter_list() {
    local clusters
    clusters=$(kind get clusters 2>/dev/null || true)

    if [[ -z "$clusters" ]]; then
        return 0
    fi

    echo "$clusters" | while read -r cluster; do
        echo "  - $cluster (context: kind-${cluster})"
    done
}

# Export functions
export -f detect_os detect_arch install_kind_macos install_kind_debian install_kind_binary prompt_install_kind
export -f adapter_check_prerequisites adapter_create adapter_get_kubeconfig
export -f adapter_destroy adapter_exists adapter_status adapter_list
