#!/bin/bash
# DevPod adapter - Full lifecycle management for DevPod deployments
#
# Usage:
#   devpod-adapter.sh <command> [OPTIONS] [sindri.yaml]
#
# Commands:
#   deploy     Create/update DevPod workspace
#   connect    SSH into workspace
#   destroy    Delete workspace and cleanup
#   plan       Show deployment plan
#   status     Show workspace status
#
# Options:
#   --config-only    Generate devcontainer.json without deploying (deploy only)
#   --output-dir     Directory for generated files (default: current directory)
#   --output-vars    Output parsed variables as JSON (deploy only)
#   --workspace-name Override workspace name from sindri.yaml
#   --force          Skip confirmation prompts (destroy only)
#   --help           Show this help message
#
# Examples:
#   devpod-adapter.sh deploy sindri.yaml
#   devpod-adapter.sh connect
#   devpod-adapter.sh destroy --force
#   devpod-adapter.sh status

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BASE_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Default values
COMMAND=""
SINDRI_YAML=""
CONFIG_ONLY=false
OUTPUT_DIR="."
OUTPUT_VARS=false
WORKSPACE_NAME_OVERRIDE=""
FORCE=false

show_help() {
    head -26 "$0" | tail -24
    exit 0
}

# Parse arguments
[[ $# -eq 0 ]] && show_help

COMMAND="$1"
shift

while [[ $# -gt 0 ]]; do
    case $1 in
        --config-only)  CONFIG_ONLY=true; shift ;;
        --output-dir)   OUTPUT_DIR="$2"; shift 2 ;;
        --output-vars)  OUTPUT_VARS=true; shift ;;
        --workspace-name) WORKSPACE_NAME_OVERRIDE="$2"; shift 2 ;;
        --force|-f)     FORCE=true; shift ;;
        --help|-h)      show_help ;;
        -*)             echo "Unknown option: $1" >&2; exit 1 ;;
        *)              SINDRI_YAML="$1"; shift ;;
    esac
done

SINDRI_YAML="${SINDRI_YAML:-sindri.yaml}"

if [[ ! -f "$SINDRI_YAML" ]]; then
    echo "Error: $SINDRI_YAML not found" >&2
    exit 1
fi

# Source common utilities
source "$BASE_DIR/docker/lib/common.sh"

# ============================================================================
# GPU Tier Mapping Functions
# ============================================================================

get_aws_gpu_instance() {
    local tier="${1:-gpu-small}"
    case "$tier" in
        gpu-small)   echo "g4dn.xlarge" ;;
        gpu-medium)  echo "g5.2xlarge" ;;
        gpu-large)   echo "g5.4xlarge" ;;
        gpu-xlarge)  echo "p4d.24xlarge" ;;
        *)           echo "g4dn.xlarge" ;;
    esac
}

get_gcp_gpu_config() {
    local tier="${1:-gpu-small}"
    # Returns: machine_type:accelerator_type:accelerator_count
    case "$tier" in
        gpu-small)   echo "n1-standard-4:nvidia-tesla-t4:1" ;;
        gpu-medium)  echo "n1-standard-8:nvidia-tesla-a10g:1" ;;
        gpu-large)   echo "g2-standard-16:nvidia-l4:1" ;;
        gpu-xlarge)  echo "a2-megagpu-16g:nvidia-a100-80gb:8" ;;
        *)           echo "n1-standard-4:nvidia-tesla-t4:1" ;;
    esac
}

get_azure_gpu_vm() {
    local tier="${1:-gpu-small}"
    case "$tier" in
        gpu-small)   echo "Standard_NC4as_T4_v3" ;;
        gpu-medium)  echo "Standard_NC8as_T4_v3" ;;
        gpu-large)   echo "Standard_NC24ads_A100_v4" ;;
        gpu-xlarge)  echo "Standard_ND96amsr_A100_v4" ;;
        *)           echo "Standard_NC4as_T4_v3" ;;
    esac
}

get_k8s_gpu_node_selector() {
    local tier="${1:-gpu-small}"
    case "$tier" in
        gpu-small)   echo "nvidia-tesla-t4" ;;
        gpu-medium)  echo "nvidia-a10g" ;;
        gpu-large)   echo "nvidia-l40s" ;;
        gpu-xlarge)  echo "nvidia-a100" ;;
        *)           echo "nvidia-tesla-t4" ;;
    esac
}

# ============================================================================
# Configuration Parsing
# ============================================================================

parse_config() {
    NAME=$(yq '.name' "$SINDRI_YAML")
    [[ -n "$WORKSPACE_NAME_OVERRIDE" ]] && NAME="$WORKSPACE_NAME_OVERRIDE"

    PROFILE=$(yq '.extensions.profile // "minimal"' "$SINDRI_YAML")
    CUSTOM_EXTENSIONS=$(yq '.extensions.active[]? // ""' "$SINDRI_YAML" | tr '\n' ',' | sed 's/,$//')

    MEMORY=$(yq '.deployment.resources.memory // "4GB"' "$SINDRI_YAML")
    CPUS=$(yq '.deployment.resources.cpus // 2' "$SINDRI_YAML")
    VOLUME_SIZE=$(yq '.deployment.volumes.workspace.size // "10GB"' "$SINDRI_YAML" | sed 's/GB//')

    GPU_ENABLED=$(yq '.deployment.resources.gpu.enabled // false' "$SINDRI_YAML")
    GPU_TIER=$(yq '.deployment.resources.gpu.tier // "gpu-small"' "$SINDRI_YAML")
    GPU_COUNT=$(yq '.deployment.resources.gpu.count // 1' "$SINDRI_YAML")

    DEVPOD_PROVIDER=$(yq '.providers.devpod.type // "docker"' "$SINDRI_YAML")

    # Provider-specific config
    case "$DEVPOD_PROVIDER" in
        kubernetes)
            K8S_NAMESPACE=$(yq '.providers.devpod.kubernetes.namespace // "devpod"' "$SINDRI_YAML")
            K8S_STORAGE_CLASS=$(yq '.providers.devpod.kubernetes.storageClass // ""' "$SINDRI_YAML")
            K8S_CONTEXT=$(yq '.providers.devpod.kubernetes.context // ""' "$SINDRI_YAML")
            # GPU node selector for k8s
            if [[ "$GPU_ENABLED" == "true" ]]; then
                K8S_GPU_NODE_SELECTOR=$(get_k8s_gpu_node_selector "$GPU_TIER")
                print_status "Using GPU node selector: accelerator=$K8S_GPU_NODE_SELECTOR"
            fi
            ;;
        aws)
            AWS_REGION=$(yq '.providers.devpod.aws.region // "us-west-2"' "$SINDRI_YAML")
            AWS_INSTANCE_TYPE=$(yq '.providers.devpod.aws.instanceType // "c5.xlarge"' "$SINDRI_YAML")
            AWS_DISK_SIZE=$(yq '.providers.devpod.aws.diskSize // 40' "$SINDRI_YAML")
            # Override instance type for GPU
            if [[ "$GPU_ENABLED" == "true" ]]; then
                AWS_INSTANCE_TYPE=$(get_aws_gpu_instance "$GPU_TIER")
                print_status "Using GPU instance: $AWS_INSTANCE_TYPE"
            fi
            ;;
        gcp)
            GCP_ZONE=$(yq '.providers.devpod.gcp.zone // "us-central1-a"' "$SINDRI_YAML")
            GCP_MACHINE_TYPE=$(yq '.providers.devpod.gcp.machineType // "e2-standard-4"' "$SINDRI_YAML")
            GCP_DISK_SIZE=$(yq '.providers.devpod.gcp.diskSize // 40' "$SINDRI_YAML")
            # Override machine type and add accelerator for GPU
            if [[ "$GPU_ENABLED" == "true" ]]; then
                local gpu_config
                gpu_config=$(get_gcp_gpu_config "$GPU_TIER")
                GCP_MACHINE_TYPE=$(echo "$gpu_config" | cut -d: -f1)
                GCP_ACCELERATOR_TYPE=$(echo "$gpu_config" | cut -d: -f2)
                GCP_ACCELERATOR_COUNT=$(echo "$gpu_config" | cut -d: -f3)
                print_status "Using GPU: $GCP_ACCELERATOR_TYPE x$GCP_ACCELERATOR_COUNT on $GCP_MACHINE_TYPE"
            fi
            ;;
        azure)
            AZURE_LOCATION=$(yq '.providers.devpod.azure.location // "eastus"' "$SINDRI_YAML")
            AZURE_VM_SIZE=$(yq '.providers.devpod.azure.vmSize // "Standard_D4s_v3"' "$SINDRI_YAML")
            AZURE_DISK_SIZE=$(yq '.providers.devpod.azure.diskSize // 40' "$SINDRI_YAML")
            # Override VM size for GPU
            if [[ "$GPU_ENABLED" == "true" ]]; then
                AZURE_VM_SIZE=$(get_azure_gpu_vm "$GPU_TIER")
                print_status "Using GPU VM: $AZURE_VM_SIZE"
            fi
            ;;
        docker|ssh|digitalocean)
            # These providers don't need additional config
            ;;
    esac
}

require_devpod() {
    if ! command -v devpod >/dev/null 2>&1; then
        print_error "DevPod CLI is not installed"
        echo "Install from: https://devpod.sh/docs/getting-started/install"
        exit 1
    fi
}

ensure_devpod_provider() {
    local provider="$1"
    if devpod provider list 2>/dev/null | grep -q "^$provider "; then
        return 0
    fi
    print_status "Adding DevPod provider: $provider"
    devpod provider add "$provider" || {
        print_error "Failed to add $provider provider"
        return 1
    }
}

configure_k8s_provider() {
    [[ "$DEVPOD_PROVIDER" != "kubernetes" ]] && return 0

    print_status "Configuring kubernetes provider..."

    if [[ -n "${K8S_CONTEXT:-}" ]]; then
        print_status "  Context: $K8S_CONTEXT"
        devpod provider set-options kubernetes -o KUBERNETES_CONTEXT="$K8S_CONTEXT" 2>/dev/null || true
    fi

    if [[ -n "${K8S_NAMESPACE:-}" ]]; then
        print_status "  Namespace: $K8S_NAMESPACE"
        devpod provider set-options kubernetes -o KUBERNETES_NAMESPACE="$K8S_NAMESPACE" 2>/dev/null || true

        local context_arg=()
        [[ -n "${K8S_CONTEXT:-}" ]] && context_arg=(--context "$K8S_CONTEXT")

        if ! kubectl "${context_arg[@]}" get namespace "$K8S_NAMESPACE" >/dev/null 2>&1; then
            print_status "Creating namespace: $K8S_NAMESPACE"
            kubectl "${context_arg[@]}" create namespace "$K8S_NAMESPACE" || true
        fi
    fi

    if [[ -n "${K8S_STORAGE_CLASS:-}" ]]; then
        print_status "  Storage class: $K8S_STORAGE_CLASS"
        devpod provider set-options kubernetes -o KUBERNETES_STORAGE_CLASS="$K8S_STORAGE_CLASS" 2>/dev/null || true
    fi
}

generate_devcontainer() {
    mkdir -p "$OUTPUT_DIR/.devcontainer"
    local MEMORY_MB
    MEMORY_MB=$(echo "$MEMORY" | sed 's/GB/*1024/;s/MB//' | bc)

    cat > "$OUTPUT_DIR/.devcontainer/devcontainer.json" << EODC
{
  "name": "${NAME}",
  "dockerFile": "../Dockerfile",
  "workspaceFolder": "/alt/home/developer/workspace",
  "workspaceMount": "source=\${localWorkspaceFolder},target=/alt/home/developer/workspace,type=bind",
  "containerEnv": {
    "INSTALL_PROFILE": "${PROFILE}",
    "CUSTOM_EXTENSIONS": "${CUSTOM_EXTENSIONS}",
    "INIT_WORKSPACE": "true"
  },
  "hostRequirements": {
    "cpus": ${CPUS},
    "memory": "${MEMORY_MB}mb",
    "storage": "${VOLUME_SIZE}gb"
  },
  "customizations": {
    "vscode": {
      "extensions": [
        "ms-vscode.vscode-typescript-next",
        "dbaeumer.vscode-eslint",
        "esbenp.prettier-vscode",
        "ms-python.python",
        "golang.go",
        "rust-lang.rust-analyzer"
      ]
    }
  },
  "features": {
    "ghcr.io/devcontainers/features/github-cli:1": {},
    "ghcr.io/devcontainers/features/docker-in-docker:2": {}
  },
  "postCreateCommand": "/docker/cli/extension-manager install-profile ${PROFILE}",
  "remoteUser": "developer",
  "containerUser": "developer",
  "mounts": ["source=sindri-home,target=/alt/home/developer,type=volume"],
  "runArgs": ["--cap-add=SYS_PTRACE", "--security-opt", "seccomp=unconfined", "--cpus=${CPUS}", "--memory=${MEMORY}"],
  "forwardPorts": [3000, 8080]
}
EODC
}

# ============================================================================
# Commands
# ============================================================================

cmd_deploy() {
    parse_config

    if [[ "$OUTPUT_VARS" == "true" ]]; then
        cat << EOJSON
{"name":"$NAME","profile":"$PROFILE","provider":"$DEVPOD_PROVIDER","memory":"$MEMORY","cpus":$CPUS,"volumeSize":$VOLUME_SIZE,"gpu_enabled":$GPU_ENABLED,"gpu_tier":"$GPU_TIER"}
EOJSON
        return 0
    fi

    generate_devcontainer

    if [[ "$CONFIG_ONLY" == "true" ]]; then
        print_success "Generated DevPod configuration at $OUTPUT_DIR/.devcontainer/"
        echo "  Workspace: $NAME"
        echo "  Provider: $DEVPOD_PROVIDER"
        echo "  Profile: $PROFILE"
        if [[ "$GPU_ENABLED" == "true" ]]; then
            echo "  GPU: $GPU_TIER (count: $GPU_COUNT)"
        fi
        return 0
    fi

    require_devpod

    echo "==> Deploying with DevPod"
    echo "  Workspace: $NAME"
    echo "  Provider: $DEVPOD_PROVIDER"
    echo "  Profile: $PROFILE"
    echo "  Resources: ${CPUS} CPUs, ${MEMORY} memory, ${VOLUME_SIZE}GB storage"

    ensure_devpod_provider "$DEVPOD_PROVIDER" || exit 1
    configure_k8s_provider

    local devpod_cmd="devpod up . --provider $DEVPOD_PROVIDER --id $NAME --ide none"
    print_status "Running: $devpod_cmd"

    if eval "$devpod_cmd"; then
        print_success "DevPod workspace '$NAME' deployed"
        echo ""
        echo "Connect:  sindri connect  OR  devpod ssh $NAME"
        echo "Status:   sindri status   OR  devpod status $NAME"
        echo "Stop:     devpod stop $NAME"
        echo "Destroy:  sindri destroy  OR  devpod delete $NAME"
    else
        print_error "DevPod deployment failed"
        echo "Check: devpod provider list"
        echo "Logs:  devpod logs $NAME"
        exit 1
    fi
}

cmd_connect() {
    parse_config
    require_devpod

    if ! devpod list 2>/dev/null | grep -q "$NAME"; then
        print_error "Workspace '$NAME' not found"
        echo "Deploy first: sindri deploy"
        exit 1
    fi

    devpod ssh "$NAME"
}

cmd_destroy() {
    parse_config

    if [[ "$FORCE" != "true" ]]; then
        print_warning "This will destroy workspace '$NAME'"
        read -p "Are you sure? (y/N) " -n 1 -r
        echo
        [[ ! $REPLY =~ ^[Yy]$ ]] && { print_status "Cancelled"; exit 0; }
    fi

    print_header "Destroying DevPod workspace: $NAME"

    if command -v devpod >/dev/null 2>&1; then
        if devpod list 2>/dev/null | grep -q "$NAME"; then
            print_status "Stopping workspace..."
            devpod stop "$NAME" 2>/dev/null || true
            print_status "Deleting workspace..."
            devpod delete "$NAME" --force 2>/dev/null || true
        else
            print_warning "Workspace '$NAME' not found in DevPod"
        fi
    fi

    if [[ -d ".devcontainer" ]]; then
        rm -rf .devcontainer
        print_status "Removed .devcontainer directory"
    fi

    # Cleanup k8s resources if using kubernetes provider
    if [[ "$DEVPOD_PROVIDER" == "kubernetes" ]] && [[ -n "${K8S_NAMESPACE:-}" ]]; then
        local context_arg=()
        [[ -n "${K8S_CONTEXT:-}" ]] && context_arg=(--context "$K8S_CONTEXT")

        if kubectl "${context_arg[@]}" get namespace "$K8S_NAMESPACE" >/dev/null 2>&1; then
            print_status "Cleaning up kubernetes resources in namespace: $K8S_NAMESPACE"
            kubectl "${context_arg[@]}" delete pods -n "$K8S_NAMESPACE" -l devpod.sh/workspace="$NAME" 2>/dev/null || true
            kubectl "${context_arg[@]}" delete pvc -n "$K8S_NAMESPACE" -l devpod.sh/workspace="$NAME" 2>/dev/null || true
        fi
    fi

    print_success "Workspace destroyed"
}

cmd_plan() {
    parse_config

    print_header "DevPod Deployment Plan"
    echo ""
    echo "Workspace:  $NAME"
    echo "Provider:   $DEVPOD_PROVIDER"
    echo "Profile:    $PROFILE"
    echo ""
    echo "Resources:"
    echo "  CPUs:     $CPUS"
    echo "  Memory:   $MEMORY"
    echo "  Storage:  ${VOLUME_SIZE}GB"
    if [[ "$GPU_ENABLED" == "true" ]]; then
        echo "  GPU:      $GPU_TIER (count: $GPU_COUNT)"
    fi
    echo ""

    case "$DEVPOD_PROVIDER" in
        kubernetes)
            echo "Kubernetes:"
            echo "  Context:       ${K8S_CONTEXT:-<current>}"
            echo "  Namespace:     ${K8S_NAMESPACE:-devpod}"
            echo "  StorageClass:  ${K8S_STORAGE_CLASS:-<default>}"
            ;;
        aws)
            echo "AWS:"
            echo "  Region:        ${AWS_REGION:-us-west-2}"
            echo "  Instance:      ${AWS_INSTANCE_TYPE:-c5.xlarge}"
            echo "  Disk:          ${AWS_DISK_SIZE:-40}GB"
            ;;
        gcp)
            echo "GCP:"
            echo "  Zone:          ${GCP_ZONE:-us-central1-a}"
            echo "  Machine:       ${GCP_MACHINE_TYPE:-e2-standard-4}"
            echo "  Disk:          ${GCP_DISK_SIZE:-40}GB"
            ;;
        azure)
            echo "Azure:"
            echo "  Location:      ${AZURE_LOCATION:-eastus}"
            echo "  VM Size:       ${AZURE_VM_SIZE:-Standard_D4s_v3}"
            echo "  Disk:          ${AZURE_DISK_SIZE:-40}GB"
            ;;
        docker)
            echo "Docker: Local container"
            ;;
        *)
            echo "Provider: $DEVPOD_PROVIDER"
            ;;
    esac

    echo ""
    echo "Actions:"
    echo "  1. Generate .devcontainer/devcontainer.json"
    echo "  2. Add '$DEVPOD_PROVIDER' provider to DevPod (if needed)"
    if [[ "$DEVPOD_PROVIDER" == "kubernetes" ]]; then
        echo "  3. Create namespace '$K8S_NAMESPACE' (if needed)"
        echo "  4. Run: devpod up . --provider $DEVPOD_PROVIDER --id $NAME"
    else
        echo "  3. Run: devpod up . --provider $DEVPOD_PROVIDER --id $NAME"
    fi
}

cmd_status() {
    parse_config
    require_devpod

    print_header "DevPod Workspace Status"
    echo ""
    echo "Workspace: $NAME"
    echo "Provider:  $DEVPOD_PROVIDER"
    echo ""

    if devpod list 2>/dev/null | grep -q "$NAME"; then
        devpod status "$NAME" 2>/dev/null || true
        echo ""

        if [[ "$DEVPOD_PROVIDER" == "kubernetes" ]] && [[ -n "${K8S_CONTEXT:-}" ]]; then
            echo "Kubernetes resources:"
            local context_arg=()
            [[ -n "${K8S_CONTEXT:-}" ]] && context_arg=(--context "$K8S_CONTEXT")
            kubectl "${context_arg[@]}" get pods -n "${K8S_NAMESPACE:-devpod}" -l devpod.sh/workspace="$NAME" 2>/dev/null || echo "  No pods found"
        fi
    else
        echo "Status: Not deployed"
        echo ""
        echo "Deploy with: sindri deploy"
    fi
}

# ============================================================================
# Main dispatch
# ============================================================================

case "$COMMAND" in
    deploy)  cmd_deploy ;;
    connect) cmd_connect ;;
    destroy) cmd_destroy ;;
    plan)    cmd_plan ;;
    status)  cmd_status ;;
    help|--help|-h) show_help ;;
    *)
        echo "Unknown command: $COMMAND" >&2
        echo "Commands: deploy, connect, destroy, plan, status"
        exit 1
        ;;
esac
