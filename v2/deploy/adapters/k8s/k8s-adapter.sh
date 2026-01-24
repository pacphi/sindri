#!/bin/bash
# k8s-adapter.sh - Local Kubernetes cluster management for Sindri
#
# This is the main dispatcher for local k8s cluster operations.
# It parses configuration from sindri.yaml and dispatches to
# provider-specific adapters (kind, k3d).
#
# Usage:
#   k8s-adapter.sh <command> [OPTIONS] [sindri.yaml]
#
# Commands:
#   create    Create a local Kubernetes cluster
#   config    Show kubeconfig context and DevPod integration info
#   destroy   Delete the cluster
#   list      List all local clusters (kind and k3d)
#   status    Show cluster status
#
# Options:
#   --provider <kind|k3d>  Override K8s provider
#   --name <name>          Override cluster name
#   --force                Skip confirmation prompts
#   --help                 Show this help

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BASE_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# Source common utilities
if [[ -f "$BASE_DIR/v2/docker/lib/common.sh" ]]; then
    source "$BASE_DIR/v2/docker/lib/common.sh"
elif [[ -f "/docker/lib/common.sh" ]]; then
    source "/docker/lib/common.sh"
else
    # Minimal fallback for standalone testing
    command_exists() { command -v "$1" >/dev/null 2>&1; }
    print_status() { echo "[INFO] $1"; }
    print_success() { echo "[SUCCESS] $1"; }
    print_warning() { echo "[WARNING] $1"; }
    print_error() { echo "[ERROR] $1" >&2; }
    print_header() { echo "==> $1"; }
fi

# Configuration globals (set by parse_config)
export K8S_PROVIDER=""
export CLUSTER_NAME=""
export K8S_VERSION=""
export NODES=""
export KIND_IMAGE=""
export KIND_CONFIG=""
export K3D_IMAGE=""
export K3D_REGISTRY=""
export K3D_REGISTRY_NAME=""
export K3D_REGISTRY_PORT=""

# Show help
show_help() {
    cat << 'EOF'
Usage: k8s-adapter.sh <command> [OPTIONS] [sindri.yaml]

Commands:
    create    Create a local Kubernetes cluster
    config    Show kubeconfig context and DevPod integration info
    destroy   Delete the cluster
    list      List all local clusters (kind and k3d)
    status    Show cluster status

Options:
    --provider <kind|k3d>  Override K8s provider (default: from config or auto-detect)
    --name <name>          Override cluster name (default: from config or sindri-local)
    --force, -f            Skip confirmation prompts
    --help, -h             Show this help

Configuration:
    If sindri.yaml is provided, reads from providers.k8s section:
      - provider: kind or k3d
      - clusterName: cluster name
      - nodes: number of nodes
      - kind.image, kind.configFile
      - k3d.image, k3d.registry.*

Examples:
    # Create cluster using sindri.yaml config
    k8s-adapter.sh create sindri.yaml

    # Create kind cluster with explicit options
    k8s-adapter.sh create --provider kind --name my-cluster

    # Show cluster context for DevPod
    k8s-adapter.sh config --name my-cluster

    # Destroy without confirmation
    k8s-adapter.sh destroy --name my-cluster --force

    # List all local clusters
    k8s-adapter.sh list
EOF
}

# Parse sindri.yaml for k8s configuration
parse_config() {
    local config_file="$1"

    if [[ ! -f "$config_file" ]]; then
        return 0
    fi

    if ! command_exists yq; then
        print_warning "yq not found, cannot parse sindri.yaml"
        return 0
    fi

    # Read k8s provider config
    K8S_PROVIDER="${K8S_PROVIDER:-$(yq '.providers.k8s.provider // ""' "$config_file" 2>/dev/null || echo "")}"
    CLUSTER_NAME="${CLUSTER_NAME:-$(yq '.providers.k8s.clusterName // ""' "$config_file" 2>/dev/null || echo "")}"

    # Fallback cluster name to project name
    if [[ -z "$CLUSTER_NAME" ]]; then
        CLUSTER_NAME=$(yq '.name // ""' "$config_file" 2>/dev/null || echo "")
    fi

    K8S_VERSION="${K8S_VERSION:-$(yq '.providers.k8s.version // "v1.31.0"' "$config_file" 2>/dev/null || echo "v1.31.0")}"
    NODES="${NODES:-$(yq '.providers.k8s.nodes // 1' "$config_file" 2>/dev/null || echo "1")}"

    # kind-specific
    KIND_IMAGE="${KIND_IMAGE:-$(yq '.providers.k8s.kind.image // ""' "$config_file" 2>/dev/null || echo "")}"
    KIND_CONFIG="${KIND_CONFIG:-$(yq '.providers.k8s.kind.configFile // ""' "$config_file" 2>/dev/null || echo "")}"

    # k3d-specific
    K3D_IMAGE="${K3D_IMAGE:-$(yq '.providers.k8s.k3d.image // ""' "$config_file" 2>/dev/null || echo "")}"
    K3D_REGISTRY="${K3D_REGISTRY:-$(yq '.providers.k8s.k3d.registry.enabled // false' "$config_file" 2>/dev/null || echo "false")}"
    K3D_REGISTRY_NAME="${K3D_REGISTRY_NAME:-$(yq '.providers.k8s.k3d.registry.name // "k3d-registry"' "$config_file" 2>/dev/null || echo "k3d-registry")}"
    K3D_REGISTRY_PORT="${K3D_REGISTRY_PORT:-$(yq '.providers.k8s.k3d.registry.port // 5000' "$config_file" 2>/dev/null || echo "5000")}"
}

# Auto-detect available provider
detect_provider() {
    if command_exists kind; then
        echo "kind"
    elif command_exists k3d; then
        echo "k3d"
    else
        echo ""
    fi
}

# Source the appropriate provider adapter
load_adapter() {
    local provider="$1"

    case "$provider" in
        kind)
            source "$SCRIPT_DIR/kind-adapter.sh"
            ;;
        k3d)
            source "$SCRIPT_DIR/k3d-adapter.sh"
            ;;
        *)
            print_error "Unknown k8s provider: $provider (use kind or k3d)"
            return 1
            ;;
    esac
}

# Main entry point
main() {
    local command="${1:-help}"
    shift || true

    # Parse arguments
    local config_file=""
    local force=""

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --provider|-p)
                K8S_PROVIDER="$2"
                shift 2
                ;;
            --name|-n)
                CLUSTER_NAME="$2"
                shift 2
                ;;
            --force|-f)
                force="--force"
                shift
                ;;
            --help|-h)
                show_help
                return 0
                ;;
            -*)
                print_error "Unknown option: $1"
                show_help
                return 1
                ;;
            *)
                # Assume it's the config file
                if [[ -f "$1" ]]; then
                    config_file="$1"
                fi
                shift
                ;;
        esac
    done

    # Parse config file if provided
    if [[ -n "$config_file" ]]; then
        parse_config "$config_file"
    fi

    # Handle list command first (doesn't need provider)
    if [[ "$command" == "list" ]]; then
        print_header "Local Kubernetes Clusters"

        local found=false

        if command_exists kind; then
            local kind_list
            kind_list=$(kind get clusters 2>/dev/null || true)
            if [[ -n "$kind_list" ]]; then
                echo ""
                print_status "kind clusters:"
                echo "$kind_list" | while read -r cluster; do
                    echo "  - $cluster (context: kind-${cluster})"
                done
                found=true
            fi
        fi

        if command_exists k3d; then
            local k3d_list
            if command_exists jq; then
                k3d_list=$(k3d cluster list -o json 2>/dev/null | jq -r '.[].name' || true)
            else
                k3d_list=$(k3d cluster list 2>/dev/null | tail -n +2 | awk '{print $1}' || true)
            fi
            if [[ -n "$k3d_list" ]]; then
                echo ""
                print_status "k3d clusters:"
                echo "$k3d_list" | while read -r cluster; do
                    [[ -n "$cluster" ]] && echo "  - $cluster (context: k3d-${cluster})"
                done
                found=true
            fi
        fi

        if [[ "$found" == "false" ]]; then
            print_status "No local k8s clusters found"
            echo ""
            print_status "Create one with: sindri k8s create --provider kind"
        fi

        return 0
    fi

    # Auto-detect provider if not set
    if [[ -z "$K8S_PROVIDER" ]]; then
        K8S_PROVIDER=$(detect_provider)
        if [[ -z "$K8S_PROVIDER" ]]; then
            print_error "No k8s provider found. Install kind or k3d."
            print_status "  kind: https://kind.sigs.k8s.io/docs/user/quick-start/#installation"
            print_status "  k3d:  https://k3d.io/#installation"
            return 1
        fi
        print_status "Auto-detected provider: $K8S_PROVIDER"
    fi

    # Set default cluster name
    CLUSTER_NAME="${CLUSTER_NAME:-sindri-local}"

    # Load the appropriate adapter
    load_adapter "$K8S_PROVIDER" || return 1

    # Dispatch command
    case "$command" in
        create)
            adapter_create
            ;;
        config)
            local context
            case "$K8S_PROVIDER" in
                kind) context="kind-${CLUSTER_NAME}" ;;
                k3d)  context="k3d-${CLUSTER_NAME}" ;;
            esac

            if ! adapter_exists "$CLUSTER_NAME"; then
                print_error "Cluster '$CLUSTER_NAME' does not exist"
                print_status "Create it with: sindri k8s create --provider $K8S_PROVIDER --name $CLUSTER_NAME"
                return 1
            fi

            print_header "Cluster Configuration"
            echo "  Name:     $CLUSTER_NAME"
            echo "  Provider: $K8S_PROVIDER"
            echo "  Context:  $context"
            echo ""
            print_status "Kubeconfig:"
            echo "  kubectl config use-context $context"
            echo ""
            print_status "DevPod integration (add to sindri.yaml):"
            echo "  providers:"
            echo "    devpod:"
            echo "      type: kubernetes"
            echo "      kubernetes:"
            echo "        context: $context"
            ;;
        destroy)
            adapter_destroy "$CLUSTER_NAME" "$force"
            ;;
        status)
            adapter_status "$CLUSTER_NAME"
            ;;
        help|--help|-h)
            show_help
            ;;
        *)
            print_error "Unknown command: $command"
            show_help
            return 1
            ;;
    esac
}

# Run main if executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
