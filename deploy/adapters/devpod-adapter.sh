#!/bin/bash
# DevPod adapter - DevContainer deployment
#
# Usage:
#   devpod-adapter.sh [OPTIONS] [sindri.yaml]
#
# Options:
#   --config-only    Generate devcontainer.json without deploying
#   --output-dir     Directory for generated files (default: current directory)
#   --output-vars    Output parsed variables for CI integration (JSON to stdout)
#   --workspace-name Override workspace name from sindri.yaml
#   --help           Show this help message
#
# Examples:
#   devpod-adapter.sh                           # Generate config using ./sindri.yaml
#   devpod-adapter.sh --config-only             # Just generate devcontainer.json
#   devpod-adapter.sh --output-dir /tmp         # Generate to specific directory

set -e

# shellcheck disable=SC2034  # May be used in future adapter implementations
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BASE_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Default values
SINDRI_YAML=""
CONFIG_ONLY=false
OUTPUT_DIR="."
OUTPUT_VARS=false
WORKSPACE_NAME_OVERRIDE=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --config-only)
            CONFIG_ONLY=true
            shift
            ;;
        --output-dir)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --output-vars)
            OUTPUT_VARS=true
            shift
            ;;
        --workspace-name)
            WORKSPACE_NAME_OVERRIDE="$2"
            shift 2
            ;;
        --help)
            head -18 "$0" | tail -16
            exit 0
            ;;
        -*)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
        *)
            SINDRI_YAML="$1"
            shift
            ;;
    esac
done

# Default sindri.yaml if not specified
SINDRI_YAML="${SINDRI_YAML:-sindri.yaml}"

if [[ ! -f "$SINDRI_YAML" ]]; then
    echo "Error: $SINDRI_YAML not found" >&2
    exit 1
fi

# Source common utilities and secrets manager
source "$BASE_DIR/docker/lib/common.sh"
if [[ "$CONFIG_ONLY" != "true" ]]; then
    source "$BASE_DIR/cli/secrets-manager"
fi

# Parse sindri.yaml - Common configuration
NAME=$(yq '.name' "$SINDRI_YAML")
# Apply workspace name override if provided
[[ -n "$WORKSPACE_NAME_OVERRIDE" ]] && NAME="$WORKSPACE_NAME_OVERRIDE"

PROFILE=$(yq '.extensions.profile // "minimal"' "$SINDRI_YAML")
CUSTOM_EXTENSIONS=$(yq '.extensions.active[]? // ""' "$SINDRI_YAML" | tr '\n' ',' | sed 's/,$//')

# Parse deployment resources
MEMORY=$(yq '.deployment.resources.memory // "4GB"' "$SINDRI_YAML")
CPUS=$(yq '.deployment.resources.cpus // 2' "$SINDRI_YAML")
VOLUME_SIZE=$(yq '.deployment.volumes.workspace.size // "10GB"' "$SINDRI_YAML" | sed 's/GB//')

# Parse GPU configuration
GPU_ENABLED=$(yq '.deployment.resources.gpu.enabled // false' "$SINDRI_YAML")
GPU_TIER=$(yq '.deployment.resources.gpu.tier // "gpu-small"' "$SINDRI_YAML")
GPU_COUNT=$(yq '.deployment.resources.gpu.count // 1' "$SINDRI_YAML")

# GPU tier to provider-specific instance mapping functions
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

# Parse DevPod provider configuration
DEVPOD_PROVIDER=$(yq '.providers.devpod.type // "docker"' "$SINDRI_YAML")

# Provider-specific configuration parsing
case "$DEVPOD_PROVIDER" in
    aws)
        AWS_REGION=$(yq '.providers.devpod.aws.region // "us-west-2"' "$SINDRI_YAML")
        AWS_INSTANCE_TYPE=$(yq '.providers.devpod.aws.instanceType // "c5.xlarge"' "$SINDRI_YAML")
        AWS_DISK_SIZE=$(yq '.providers.devpod.aws.diskSize // 40' "$SINDRI_YAML")
        AWS_USE_SPOT=$(yq '.providers.devpod.aws.useSpot // false' "$SINDRI_YAML")
        AWS_SUBNET_ID=$(yq '.providers.devpod.aws.subnetId // ""' "$SINDRI_YAML")
        AWS_SECURITY_GROUP=$(yq '.providers.devpod.aws.securityGroupId // ""' "$SINDRI_YAML")
        # Override instance type for GPU
        if [[ "$GPU_ENABLED" == "true" ]]; then
            AWS_INSTANCE_TYPE=$(get_aws_gpu_instance "$GPU_TIER")
            echo "Using GPU instance: $AWS_INSTANCE_TYPE" >&2
        fi
        ;;
    gcp)
        GCP_PROJECT=$(yq '.providers.devpod.gcp.project // ""' "$SINDRI_YAML")
        GCP_ZONE=$(yq '.providers.devpod.gcp.zone // "us-central1-a"' "$SINDRI_YAML")
        GCP_MACHINE_TYPE=$(yq '.providers.devpod.gcp.machineType // "e2-standard-4"' "$SINDRI_YAML")
        GCP_DISK_SIZE=$(yq '.providers.devpod.gcp.diskSize // 40' "$SINDRI_YAML")
        GCP_DISK_TYPE=$(yq '.providers.devpod.gcp.diskType // "pd-balanced"' "$SINDRI_YAML")
        GCP_ACCELERATOR_TYPE=""
        GCP_ACCELERATOR_COUNT=0
        # Override machine type and add accelerator for GPU
        if [[ "$GPU_ENABLED" == "true" ]]; then
            GPU_CONFIG=$(get_gcp_gpu_config "$GPU_TIER")
            GCP_MACHINE_TYPE=$(echo "$GPU_CONFIG" | cut -d: -f1)
            GCP_ACCELERATOR_TYPE=$(echo "$GPU_CONFIG" | cut -d: -f2)
            GCP_ACCELERATOR_COUNT=$(echo "$GPU_CONFIG" | cut -d: -f3)
            echo "Using GPU: $GCP_ACCELERATOR_TYPE x$GCP_ACCELERATOR_COUNT on $GCP_MACHINE_TYPE" >&2
        fi
        ;;
    azure)
        AZURE_SUBSCRIPTION=$(yq '.providers.devpod.azure.subscription // ""' "$SINDRI_YAML")
        AZURE_RESOURCE_GROUP=$(yq '.providers.devpod.azure.resourceGroup // "devpod-resources"' "$SINDRI_YAML")
        AZURE_LOCATION=$(yq '.providers.devpod.azure.location // "eastus"' "$SINDRI_YAML")
        AZURE_VM_SIZE=$(yq '.providers.devpod.azure.vmSize // "Standard_D4s_v3"' "$SINDRI_YAML")
        AZURE_DISK_SIZE=$(yq '.providers.devpod.azure.diskSize // 40' "$SINDRI_YAML")
        # Override VM size for GPU
        if [[ "$GPU_ENABLED" == "true" ]]; then
            AZURE_VM_SIZE=$(get_azure_gpu_vm "$GPU_TIER")
            echo "Using GPU VM: $AZURE_VM_SIZE" >&2
        fi
        ;;
    digitalocean)
        DO_REGION=$(yq '.providers.devpod.digitalocean.region // "nyc3"' "$SINDRI_YAML")
        DO_SIZE=$(yq '.providers.devpod.digitalocean.size // "s-4vcpu-8gb"' "$SINDRI_YAML")
        DO_DISK_SIZE=$(yq '.providers.devpod.digitalocean.diskSize // 0' "$SINDRI_YAML")
        ;;
    kubernetes)
        K8S_NAMESPACE=$(yq '.providers.devpod.kubernetes.namespace // "devpod"' "$SINDRI_YAML")
        K8S_STORAGE_CLASS=$(yq '.providers.devpod.kubernetes.storageClass // ""' "$SINDRI_YAML")
        K8S_CONTEXT=$(yq '.providers.devpod.kubernetes.context // ""' "$SINDRI_YAML")
        K8S_GPU_NODE_SELECTOR=""
        # shellcheck disable=SC2034  # Used in provider.yaml generation
        K8S_GPU_RESOURCE_KEY="nvidia.com/gpu"
        # Add GPU node selector for GPU
        if [[ "$GPU_ENABLED" == "true" ]]; then
            K8S_GPU_NODE_SELECTOR=$(get_k8s_gpu_node_selector "$GPU_TIER")
            echo "Using GPU node selector: accelerator=$K8S_GPU_NODE_SELECTOR" >&2
        fi
        ;;
    ssh)
        SSH_HOST=$(yq '.providers.devpod.ssh.host // ""' "$SINDRI_YAML")
        SSH_USER=$(yq '.providers.devpod.ssh.user // "root"' "$SINDRI_YAML")
        SSH_PORT=$(yq '.providers.devpod.ssh.port // 22' "$SINDRI_YAML")
        SSH_KEY_PATH=$(yq '.providers.devpod.ssh.keyPath // "~/.ssh/id_rsa"' "$SINDRI_YAML")
        ;;
    docker)
        DOCKER_HOST=$(yq '.providers.devpod.docker.dockerHost // ""' "$SINDRI_YAML")
        ;;
esac

# Output variables for CI integration if requested
if [[ "$OUTPUT_VARS" == "true" ]]; then
    # Build provider-specific JSON
    PROVIDER_CONFIG="{}"
    case "$DEVPOD_PROVIDER" in
        aws)
            PROVIDER_CONFIG=$(cat << EOJSON
{
      "region": "$AWS_REGION",
      "instanceType": "$AWS_INSTANCE_TYPE",
      "diskSize": $AWS_DISK_SIZE,
      "useSpot": $AWS_USE_SPOT
    }
EOJSON
)
            ;;
        gcp)
            PROVIDER_CONFIG=$(cat << EOJSON
{
      "zone": "$GCP_ZONE",
      "machineType": "$GCP_MACHINE_TYPE",
      "diskSize": $GCP_DISK_SIZE,
      "diskType": "$GCP_DISK_TYPE"
    }
EOJSON
)
            ;;
        azure)
            PROVIDER_CONFIG=$(cat << EOJSON
{
      "location": "$AZURE_LOCATION",
      "vmSize": "$AZURE_VM_SIZE",
      "diskSize": $AZURE_DISK_SIZE
    }
EOJSON
)
            ;;
        digitalocean)
            PROVIDER_CONFIG=$(cat << EOJSON
{
      "region": "$DO_REGION",
      "size": "$DO_SIZE"
    }
EOJSON
)
            ;;
        kubernetes)
            PROVIDER_CONFIG=$(cat << EOJSON
{
      "namespace": "$K8S_NAMESPACE"
    }
EOJSON
)
            ;;
    esac

    cat << EOJSON
{
  "name": "$NAME",
  "profile": "$PROFILE",
  "provider": "$DEVPOD_PROVIDER",
  "memory": "$MEMORY",
  "cpus": $CPUS,
  "volumeSize": $VOLUME_SIZE,
  "gpu_enabled": $GPU_ENABLED,
  "gpu_tier": "$GPU_TIER",
  "gpu_count": $GPU_COUNT,
  "providerConfig": $PROVIDER_CONFIG
}
EOJSON
    exit 0
fi

# Resolve secrets (skip in config-only mode)
if [[ "$CONFIG_ONLY" != "true" ]]; then
    print_status "Resolving secrets..."
    secrets_resolve_all "$SINDRI_YAML" || true
fi

# Create .devcontainer directory in output location
mkdir -p "$OUTPUT_DIR/.devcontainer"

# Convert memory string to numeric for hostRequirements (e.g., "4GB" -> 4096)
MEMORY_MB=$(echo "$MEMORY" | sed 's/GB/*1024/;s/MB//' | bc)

# Generate devcontainer.json with secrets and resource configuration
{
cat << EODC
{
  "name": "${NAME}",
  "dockerFile": "../Dockerfile",
  "workspaceFolder": "/alt/home/developer/workspace",
  "workspaceMount": "source=\${localWorkspaceFolder},target=/alt/home/developer/workspace,type=bind",

EODC

# Add secrets as containerEnv (skip in config-only mode)
if [[ "$CONFIG_ONLY" != "true" ]]; then
    secrets_generate_devcontainer_env
else
    # Add environment variables including profile and extensions
    cat << EODC
  "containerEnv": {
    "INSTALL_PROFILE": "${PROFILE}",
    "CUSTOM_EXTENSIONS": "${CUSTOM_EXTENSIONS}",
    "INIT_WORKSPACE": "true"
  }
EODC
fi

cat << EODC
,

  "hostRequirements": {
    "cpus": ${CPUS},
    "memory": "${MEMORY_MB}mb",
    "storage": "${VOLUME_SIZE}gb"$(if [[ "$GPU_ENABLED" == "true" ]]; then echo ',
    "gpu": "optional"'; fi)
  },

  "customizations": {
    "vscode": {
      "extensions": [
        "ms-vscode.vscode-typescript-next",
        "dbaeumer.vscode-eslint",
        "esbenp.prettier-vscode",
        "ms-python.python",
        "ms-python.vscode-pylance",
        "golang.go",
        "rust-lang.rust-analyzer"
      ],
      "settings": {
        "terminal.integrated.defaultProfile.linux": "bash",
        "terminal.integrated.profiles.linux": {
          "bash": {
            "path": "/bin/bash",
            "icon": "terminal-bash"
          }
        }
      }
    }
  },

  "features": {
    "ghcr.io/devcontainers/features/github-cli:1": {},
    "ghcr.io/devcontainers/features/docker-in-docker:2": {}
  },

  "postCreateCommand": "/docker/cli/extension-manager install-profile ${PROFILE}",
  "postStartCommand": "echo 'Welcome to Sindri DevContainer!'",

  "remoteUser": "developer",
  "containerUser": "developer",

  "mounts": [
    "source=sindri-home,target=/alt/home/developer,type=volume"
  ],

  "runArgs": [
    "--cap-add=SYS_PTRACE",
    "--security-opt", "seccomp=unconfined",
    "--cpus=${CPUS}",
    "--memory=${MEMORY}"$(if [[ "$GPU_ENABLED" == "true" ]]; then echo ',
    "--gpus", "all"'; fi)
  ],

  "forwardPorts": [3000, 8080],

  "portsAttributes": {
    "3000": {
      "label": "Application",
      "onAutoForward": "notify"
    },
    "8080": {
      "label": "API",
      "onAutoForward": "silent"
    }
  }
}
EODC
} > "$OUTPUT_DIR/.devcontainer/devcontainer.json"

# Generate provider.yaml for DevPod with provider-specific options
{
cat << EOPY
# Sindri DevPod provider configuration
# Provider type: ${DEVPOD_PROVIDER}
name: sindri-provider
version: v1.0.0
description: Sindri development environment provider (${DEVPOD_PROVIDER})

agent:
  path: \${DEVPOD}
  driver: docker
  docker:
    buildRepository: sindri-devpod

options:
  PROFILE:
    description: Extension profile to install
    default: "${PROFILE}"
    enum: ["minimal", "fullstack", "ai-dev", "systems", "enterprise"]

  CPUS:
    description: Number of CPUs
    default: "${CPUS}"

  MEMORY:
    description: Memory allocation
    default: "${MEMORY}"

  WORKSPACE_SIZE:
    description: Workspace volume size in GB
    default: "${VOLUME_SIZE}"

EOPY

# Add provider-specific options
case "$DEVPOD_PROVIDER" in
    aws)
        cat << EOPY
  # AWS EC2 specific options
  AWS_REGION:
    description: AWS region
    default: "${AWS_REGION}"

  AWS_INSTANCE_TYPE:
    description: EC2 instance type
    default: "${AWS_INSTANCE_TYPE}"

  AWS_DISK_SIZE:
    description: Root volume size in GB
    default: "${AWS_DISK_SIZE}"

  AWS_USE_SPOT:
    description: Use spot instances for cost savings
    default: "${AWS_USE_SPOT}"
EOPY
        [[ -n "$AWS_SUBNET_ID" ]] && echo "
  AWS_SUBNET_ID:
    description: VPC subnet ID
    default: \"${AWS_SUBNET_ID}\""
        [[ -n "$AWS_SECURITY_GROUP" ]] && echo "
  AWS_SECURITY_GROUP:
    description: Security group ID
    default: \"${AWS_SECURITY_GROUP}\""
        ;;
    gcp)
        cat << EOPY
  # GCP Compute Engine specific options
  GCP_ZONE:
    description: GCP zone
    default: "${GCP_ZONE}"

  GCP_MACHINE_TYPE:
    description: GCE machine type
    default: "${GCP_MACHINE_TYPE}"

  GCP_DISK_SIZE:
    description: Boot disk size in GB
    default: "${GCP_DISK_SIZE}"

  GCP_DISK_TYPE:
    description: Persistent disk type
    default: "${GCP_DISK_TYPE}"
    enum: ["pd-standard", "pd-balanced", "pd-ssd"]
EOPY
        [[ -n "$GCP_PROJECT" ]] && echo "
  GCP_PROJECT:
    description: GCP project ID
    default: \"${GCP_PROJECT}\""
        ;;
    azure)
        cat << EOPY
  # Azure VM specific options
  AZURE_LOCATION:
    description: Azure region
    default: "${AZURE_LOCATION}"

  AZURE_VM_SIZE:
    description: VM size
    default: "${AZURE_VM_SIZE}"

  AZURE_DISK_SIZE:
    description: OS disk size in GB
    default: "${AZURE_DISK_SIZE}"

  AZURE_RESOURCE_GROUP:
    description: Resource group name
    default: "${AZURE_RESOURCE_GROUP}"
EOPY
        [[ -n "$AZURE_SUBSCRIPTION" ]] && echo "
  AZURE_SUBSCRIPTION:
    description: Azure subscription ID
    default: \"${AZURE_SUBSCRIPTION}\""
        ;;
    digitalocean)
        cat << EOPY
  # DigitalOcean Droplet specific options
  DO_REGION:
    description: DigitalOcean region
    default: "${DO_REGION}"

  DO_SIZE:
    description: Droplet size
    default: "${DO_SIZE}"
EOPY
        [[ "$DO_DISK_SIZE" -gt 0 ]] && echo "
  DO_DISK_SIZE:
    description: Block storage size in GB
    default: \"${DO_DISK_SIZE}\""
        ;;
    kubernetes)
        cat << EOPY
  # Kubernetes pod specific options
  K8S_NAMESPACE:
    description: Kubernetes namespace
    default: "${K8S_NAMESPACE}"
EOPY
        [[ -n "$K8S_STORAGE_CLASS" ]] && echo "
  K8S_STORAGE_CLASS:
    description: Storage class for persistent volumes
    default: \"${K8S_STORAGE_CLASS}\""
        [[ -n "$K8S_CONTEXT" ]] && echo "
  K8S_CONTEXT:
    description: Kubernetes context to use
    default: \"${K8S_CONTEXT}\""
        ;;
    ssh)
        cat << EOPY
  # SSH provider specific options
  SSH_HOST:
    description: SSH host address
    default: "${SSH_HOST}"

  SSH_USER:
    description: SSH user
    default: "${SSH_USER}"

  SSH_PORT:
    description: SSH port
    default: "${SSH_PORT}"

  SSH_KEY_PATH:
    description: Path to SSH private key
    default: "${SSH_KEY_PATH}"
EOPY
        ;;
    docker)
        [[ -n "$DOCKER_HOST" ]] && cat << EOPY
  # Docker provider specific options
  DOCKER_HOST:
    description: Docker host URL
    default: "${DOCKER_HOST}"
EOPY
        ;;
esac

cat << EOPY

exec:
  command: |-
    docker exec -it \${CONTAINER_ID} \${COMMAND}

  init: |-
    echo "Initializing Sindri workspace..."
    /docker/cli/extension-manager install-profile \${PROFILE}

  shutdown: |-
    echo "Shutting down Sindri workspace..."
EOPY
} > "$OUTPUT_DIR/.devcontainer/provider.yaml"

# If config-only mode, just report success and exit
if [[ "$CONFIG_ONLY" == "true" ]]; then
    echo "==> Generated DevPod configuration at $OUTPUT_DIR/.devcontainer/"
    echo "    Workspace name: $NAME"
    echo "    Provider: $DEVPOD_PROVIDER"
    echo "    Profile: $PROFILE"
    echo "    Resources: ${CPUS} CPUs, ${MEMORY} memory, ${VOLUME_SIZE}GB storage"
    if [[ "$GPU_ENABLED" == "true" ]]; then
        echo "    GPU: $GPU_TIER (count: $GPU_COUNT)"
    fi
    exit 0
fi

echo "==> DevPod configuration created"
echo ""
echo "Configuration:"
echo "  Provider: $DEVPOD_PROVIDER"
echo "  Profile: $PROFILE"
echo "  Resources: ${CPUS} CPUs, ${MEMORY} memory, ${VOLUME_SIZE}GB storage"
echo ""
echo "To use with DevPod:"
echo "  1. Install DevPod: https://devpod.sh/docs/getting-started/install"
echo "  2. Create workspace: devpod up . --provider $DEVPOD_PROVIDER"
echo ""
echo "To use with VS Code:"
echo "  1. Install 'Dev Containers' extension"
echo "  2. Open folder in container: Ctrl+Shift+P -> 'Dev Containers: Open Folder in Container'"
echo ""
echo "To use with GitHub Codespaces:"
echo "  1. Push to GitHub repository"
echo "  2. Create codespace from repository"
