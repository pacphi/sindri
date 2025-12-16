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
#   --config-only        Generate devcontainer.json without deploying (deploy only)
#   --output-dir         Directory for generated files (default: current directory)
#   --output-vars        Output parsed variables as JSON (deploy only)
#   --workspace-name     Override workspace name from sindri.yaml
#   --build-repository   Docker registry for image push (required for non-local K8s)
#   --skip-build         Skip image build (use existing image)
#   --image              Specify image to use (overrides build; use with --skip-build)
#   --force              Skip confirmation prompts (destroy only)
#   --help               Show this help message
#
# Environment Variables:
#   DOCKER_USERNAME      Docker registry username (or use .env)
#   DOCKER_PASSWORD      Docker registry password (or use .env)
#   DOCKER_REGISTRY      Docker registry URL (default: docker.io)
#
# Examples:
#   devpod-adapter.sh deploy sindri.yaml
#   devpod-adapter.sh deploy --build-repository ghcr.io/myorg/sindri
#   devpod-adapter.sh connect
#   devpod-adapter.sh destroy --force
#   devpod-adapter.sh status

set -e

# Source common adapter functions
# shellcheck source=adapter-common.sh
source "$(dirname "${BASH_SOURCE[0]}")/adapter-common.sh"

# Initialize adapter
adapter_init "${BASH_SOURCE[0]}"

# DevPod-specific defaults
# shellcheck disable=SC2034  # Used via indirect expansion in adapter_parse_base_config
WORKSPACE_NAME_OVERRIDE=""
BUILD_REPOSITORY=""
SKIP_BUILD=false
IMAGE_OVERRIDE=""

# Show help wrapper
show_help() {
    adapter_show_help "$0" 34
}

# Parse command
if ! adapter_parse_command "$@"; then
    show_help
fi
set -- "${REMAINING_ARGS[@]}"

# Parse arguments
# shellcheck disable=SC2034  # Variables used by adapter_parse_base_config or sourced scripts
while [[ $# -gt 0 ]]; do
    case $1 in
        --config-only)  CONFIG_ONLY=true; shift ;;
        --output-dir)   OUTPUT_DIR="$2"; shift 2 ;;
        --output-vars)  OUTPUT_VARS=true; shift ;;
        --workspace-name) WORKSPACE_NAME_OVERRIDE="$2"; shift 2 ;;
        --build-repository) BUILD_REPOSITORY="$2"; shift 2 ;;
        --skip-build)   SKIP_BUILD=true; shift ;;
        --image)        IMAGE_OVERRIDE="$2"; shift 2 ;;
        --ci-mode)      CI_MODE=true; shift ;;
        --force|-f)     FORCE=true; shift ;;
        --help|-h)      show_help ;;
        -*)             adapter_unknown_option "$1" ;;
        *)              SINDRI_YAML="$1"; shift ;;
    esac
done

# Validate config file
adapter_validate_config

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
    # Parse base configuration
    adapter_parse_base_config "WORKSPACE_NAME_OVERRIDE"

    # DevPod provider type
    DEVPOD_PROVIDER=$(yq '.providers.devpod.type // "docker"' "$SINDRI_YAML")

    # Build repository from config (CLI flag takes precedence)
    if [[ -z "$BUILD_REPOSITORY" ]]; then
        BUILD_REPOSITORY=$(yq '.providers.devpod.buildRepository // ""' "$SINDRI_YAML")
    fi

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

# ============================================================================
# Image Building and Loading
# ============================================================================

# Detect if running against a local Kubernetes cluster (kind or k3d)
# Returns: "kind", "k3d", or "" (empty for non-local/unknown)
detect_local_cluster() {
    local context_arg=()
    [[ -n "${K8S_CONTEXT:-}" ]] && context_arg=(--context "$K8S_CONTEXT")

    # Check for kind cluster
    if command -v kind &>/dev/null; then
        local current_context
        current_context=$(kubectl "${context_arg[@]}" config current-context 2>/dev/null || echo "")
        if [[ "$current_context" == kind-* ]]; then
            # Extract cluster name from context (kind-<cluster-name>)
            local cluster_name="${current_context#kind-}"
            if kind get clusters 2>/dev/null | grep -q "^${cluster_name}$"; then
                echo "kind:$cluster_name"
                return 0
            fi
        fi
        # Also check if any kind cluster exists matching the context
        local kind_clusters
        kind_clusters=$(kind get clusters 2>/dev/null || true)
        if [[ -n "$kind_clusters" ]]; then
            while read -r cluster; do
                if [[ "$current_context" == "kind-$cluster" ]]; then
                    echo "kind:$cluster"
                    return 0
                fi
            done <<< "$kind_clusters"
        fi
    fi

    # Check for k3d cluster
    if command -v k3d &>/dev/null; then
        local current_context
        current_context=$(kubectl "${context_arg[@]}" config current-context 2>/dev/null || echo "")
        if [[ "$current_context" == k3d-* ]]; then
            local cluster_name="${current_context#k3d-}"
            if k3d cluster list 2>/dev/null | grep -q "^${cluster_name}"; then
                echo "k3d:$cluster_name"
                return 0
            fi
        fi
    fi

    echo ""
}

# Get Docker registry credentials from environment, .env files, or Docker config
# Sets: DOCKER_USERNAME, DOCKER_PASSWORD, DOCKER_REGISTRY
get_docker_credentials() {
    # Priority: environment > .env.local > .env > ~/.docker/config.json

    # Check environment variables first
    if [[ -n "${DOCKER_USERNAME:-}" ]] && [[ -n "${DOCKER_PASSWORD:-}" ]]; then
        print_status "  Using Docker credentials from environment"
        return 0
    fi

    # Check .env.local
    if [[ -f .env.local ]]; then
        if grep -q "^DOCKER_USERNAME=" .env.local 2>/dev/null; then
            DOCKER_USERNAME=$(grep "^DOCKER_USERNAME=" .env.local | head -1 | cut -d= -f2- | sed -e 's/^"//' -e 's/"$//')
            DOCKER_PASSWORD=$(grep "^DOCKER_PASSWORD=" .env.local | head -1 | cut -d= -f2- | sed -e 's/^"//' -e 's/"$//')
            DOCKER_REGISTRY=$(grep "^DOCKER_REGISTRY=" .env.local | head -1 | cut -d= -f2- | sed -e 's/^"//' -e 's/"$//' || echo "")
            if [[ -n "$DOCKER_USERNAME" ]] && [[ -n "$DOCKER_PASSWORD" ]]; then
                print_status "  Using Docker credentials from .env.local"
                return 0
            fi
        fi
    fi

    # Check .env
    if [[ -f .env ]]; then
        if grep -q "^DOCKER_USERNAME=" .env 2>/dev/null; then
            DOCKER_USERNAME=$(grep "^DOCKER_USERNAME=" .env | head -1 | cut -d= -f2- | sed -e 's/^"//' -e 's/"$//')
            DOCKER_PASSWORD=$(grep "^DOCKER_PASSWORD=" .env | head -1 | cut -d= -f2- | sed -e 's/^"//' -e 's/"$//')
            DOCKER_REGISTRY=$(grep "^DOCKER_REGISTRY=" .env | head -1 | cut -d= -f2- | sed -e 's/^"//' -e 's/"$//' || echo "")
            if [[ -n "$DOCKER_USERNAME" ]] && [[ -n "$DOCKER_PASSWORD" ]]; then
                print_status "  Using Docker credentials from .env"
                return 0
            fi
        fi
    fi

    # Check if already logged in via Docker config
    if [[ -f ~/.docker/config.json ]]; then
        local registry_host
        registry_host=$(echo "$BUILD_REPOSITORY" | cut -d/ -f1)
        if jq -e ".auths.\"$registry_host\" // .auths.\"https://$registry_host\"" ~/.docker/config.json &>/dev/null; then
            print_status "  Using existing Docker login for $registry_host"
            return 0
        fi
    fi

    return 1
}

# Login to Docker registry
docker_registry_login() {
    local registry_host
    registry_host=$(echo "$BUILD_REPOSITORY" | cut -d/ -f1)

    # Special handling for common registries
    case "$registry_host" in
        ghcr.io)
            if [[ -n "${GITHUB_TOKEN:-}" ]]; then
                print_status "Logging in to GitHub Container Registry..."
                echo "$GITHUB_TOKEN" | docker login ghcr.io -u "${GITHUB_ACTOR:-git}" --password-stdin
                return $?
            fi
            ;;
        *.dkr.ecr.*.amazonaws.com)
            if command -v aws &>/dev/null; then
                print_status "Logging in to Amazon ECR..."
                local region
                region=$(echo "$registry_host" | sed 's/.*\.dkr\.ecr\.\([^.]*\)\.amazonaws\.com/\1/')
                aws ecr get-login-password --region "$region" | docker login --username AWS --password-stdin "$registry_host"
                return $?
            fi
            ;;
        *.gcr.io|gcr.io)
            if [[ -n "${GOOGLE_APPLICATION_CREDENTIALS:-}" ]]; then
                print_status "Logging in to Google Container Registry..."
                cat "$GOOGLE_APPLICATION_CREDENTIALS" | docker login -u _json_key --password-stdin "https://$registry_host"
                return $?
            fi
            ;;
    esac

    # Generic registry login
    if [[ -n "${DOCKER_USERNAME:-}" ]] && [[ -n "${DOCKER_PASSWORD:-}" ]]; then
        local registry="${DOCKER_REGISTRY:-$registry_host}"
        print_status "Logging in to Docker registry: $registry"
        echo "$DOCKER_PASSWORD" | docker login "$registry" -u "$DOCKER_USERNAME" --password-stdin
        return $?
    fi

    print_warning "No Docker credentials found for $registry_host"
    return 1
}

# Build the Sindri Docker image
build_image() {
    local image_tag="$1"

    if [[ ! -f "$BASE_DIR/Dockerfile" ]]; then
        print_error "Dockerfile not found at $BASE_DIR/Dockerfile" >&2
        return 1
    fi

    print_status "Building Docker image: $image_tag" >&2
    docker build -t "$image_tag" -f "$BASE_DIR/Dockerfile" "$BASE_DIR"
}

# Push image to registry
push_image() {
    local image_tag="$1"

    print_status "Pushing image to registry: $image_tag" >&2
    docker push "$image_tag"
}

# Load image into local Kubernetes cluster (kind or k3d)
load_image_local() {
    local image_tag="$1"
    local cluster_info="$2"

    local cluster_type="${cluster_info%%:*}"
    local cluster_name="${cluster_info#*:}"

    case "$cluster_type" in
        kind)
            print_status "Loading image into kind cluster: $cluster_name" >&2
            kind load docker-image "$image_tag" --name "$cluster_name"
            ;;
        k3d)
            print_status "Loading image into k3d cluster: $cluster_name" >&2
            k3d image import "$image_tag" --cluster "$cluster_name"
            ;;
        *)
            print_error "Unknown local cluster type: $cluster_type" >&2
            return 1
            ;;
    esac
}

# Prepare image for DevPod deployment
# Returns the image tag to use in devcontainer.json
prepare_image() {
    local image_tag="sindri:latest"

    # If explicit image is provided, use it directly
    if [[ -n "$IMAGE_OVERRIDE" ]]; then
        print_status "Using specified image: $IMAGE_OVERRIDE" >&2
        echo "$IMAGE_OVERRIDE"
        return 0
    fi

    if [[ "$SKIP_BUILD" == "true" ]]; then
        print_status "Skipping image build (--skip-build)" >&2
        echo "$image_tag"
        return 0
    fi

    # For docker provider, no special handling needed (uses Dockerfile directly)
    if [[ "$DEVPOD_PROVIDER" == "docker" ]]; then
        echo ""  # Empty means use dockerFile in devcontainer.json
        return 0
    fi

    # Check for local cluster (kind/k3d)
    local local_cluster
    local_cluster=$(detect_local_cluster)

    if [[ -n "$local_cluster" ]]; then
        print_status "Detected local Kubernetes cluster: $local_cluster" >&2

        # Build locally and load into cluster
        build_image "$image_tag" || return 1
        load_image_local "$image_tag" "$local_cluster" || return 1

        echo "$image_tag"
        return 0
    fi

    # For cloud/external Kubernetes - require build repository
    if [[ "$DEVPOD_PROVIDER" == "kubernetes" ]] || [[ "$DEVPOD_PROVIDER" == "aws" ]] || \
       [[ "$DEVPOD_PROVIDER" == "gcp" ]] || [[ "$DEVPOD_PROVIDER" == "azure" ]]; then

        if [[ -z "$BUILD_REPOSITORY" ]]; then
            print_error "Build repository required for $DEVPOD_PROVIDER provider" >&2
            echo "" >&2
            echo "Options:" >&2
            echo "  1. CLI flag:    sindri deploy --build-repository ghcr.io/myorg/sindri" >&2
            echo "  2. sindri.yaml: providers.devpod.buildRepository: ghcr.io/myorg/sindri" >&2
            echo "" >&2
            echo "Also ensure Docker credentials are available:" >&2
            echo "  - Set DOCKER_USERNAME and DOCKER_PASSWORD environment variables" >&2
            echo "  - Or add them to .env or .env.local" >&2
            echo "  - Or run 'docker login' for your registry" >&2
            return 1
        fi

        image_tag="$BUILD_REPOSITORY:latest"

        # Get credentials and login
        if ! get_docker_credentials; then
            if ! docker_registry_login; then
                print_error "Failed to authenticate with Docker registry" >&2
                return 1
            fi
        fi

        # Build and push
        build_image "$image_tag" || return 1
        push_image "$image_tag" || return 1

        echo "$image_tag"
        return 0
    fi

    # Default: use Dockerfile
    echo ""
}

generate_devcontainer() {
    local image_tag="${1:-}"

    mkdir -p "$OUTPUT_DIR/.devcontainer"
    local MEMORY_MB
    MEMORY_MB=$(echo "$MEMORY" | sed 's/GB/*1024/;s/MB//' | bc)

    # Get skip_auto_install value
    local skip_auto_install
    skip_auto_install=$(adapter_get_skip_auto_install)

    # Determine image source line
    local image_source
    if [[ -n "$image_tag" ]]; then
        image_source="\"image\": \"${image_tag}\","
    else
        image_source="\"dockerFile\": \"../Dockerfile\","
    fi

    cat > "$OUTPUT_DIR/.devcontainer/devcontainer.json" << EODC
{
  "name": "${NAME}",
  ${image_source}
  "workspaceFolder": "/alt/home/developer/workspace",
  "workspaceMount": "source=\${localWorkspaceFolder},target=/alt/home/developer/workspace,type=bind",
  "containerEnv": {
    "HOME": "/alt/home/developer",
    "ALT_HOME": "/alt/home/developer",
    "WORKSPACE": "/alt/home/developer/workspace",
    "MISE_DATA_DIR": "/alt/home/developer/.local/share/mise",
    "MISE_CONFIG_DIR": "/alt/home/developer/.config/mise",
    "MISE_CACHE_DIR": "/alt/home/developer/.cache/mise",
    "MISE_STATE_DIR": "/alt/home/developer/.local/state/mise",
    "INSTALL_PROFILE": "${PROFILE}",
    "CUSTOM_EXTENSIONS": "${CUSTOM_EXTENSIONS}",
    "ADDITIONAL_EXTENSIONS": "${ADDITIONAL_EXTENSIONS}",
    "SKIP_AUTO_INSTALL": "${skip_auto_install}",
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
      ],
      "settings": {
        "terminal.integrated.defaultProfile.linux": "bash"
      }
    }
  },
  "postCreateCommand": "/docker/cli/extension-manager install-profile ${PROFILE}",
  "remoteUser": "developer",
  "containerUser": "developer"
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
{"name":"$NAME","profile":"$PROFILE","provider":"$DEVPOD_PROVIDER","memory":"$MEMORY","cpus":$CPUS,"volumeSize":$VOLUME_SIZE,"gpu_enabled":$GPU_ENABLED,"gpu_tier":"$GPU_TIER","buildRepository":"$BUILD_REPOSITORY"}
EOJSON
        return 0
    fi

    print_header "Deploying with DevPod"
    echo "  Workspace: $NAME"
    echo "  Provider: $DEVPOD_PROVIDER"
    echo "  Profile: $PROFILE"
    echo "  Resources: ${CPUS} CPUs, ${MEMORY} memory, ${VOLUME_SIZE}GB storage"
    if [[ -n "$BUILD_REPOSITORY" ]]; then
        echo "  Build Repository: $BUILD_REPOSITORY"
    fi

    # Prepare image (build/push/load as needed)
    local image_tag
    if ! image_tag=$(prepare_image); then
        print_error "Failed to prepare image"
        exit 1
    fi

    # Generate devcontainer configuration
    generate_devcontainer "$image_tag"

    if [[ "$CONFIG_ONLY" == "true" ]]; then
        print_success "Generated DevPod configuration at $OUTPUT_DIR/.devcontainer/"
        echo "  Workspace: $NAME"
        echo "  Provider: $DEVPOD_PROVIDER"
        echo "  Profile: $PROFILE"
        if [[ -n "$image_tag" ]]; then
            echo "  Image: $image_tag"
        else
            echo "  Image: (build from Dockerfile)"
        fi
        if [[ "$GPU_ENABLED" == "true" ]]; then
            echo "  GPU: $GPU_TIER (count: $GPU_COUNT)"
        fi
        return 0
    fi

    require_devpod

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

    # Show image strategy
    echo "Image Strategy:"
    case "$DEVPOD_PROVIDER" in
        docker)
            echo "  Build from Dockerfile (local Docker)"
            ;;
        kubernetes)
            local local_cluster
            local_cluster=$(detect_local_cluster)
            if [[ -n "$local_cluster" ]]; then
                echo "  Local cluster detected: $local_cluster"
                echo "  → Build locally and load into cluster"
            elif [[ -n "$BUILD_REPOSITORY" ]]; then
                echo "  Build repository: $BUILD_REPOSITORY"
                echo "  → Build and push to registry"
            else
                echo "  ⚠ No build repository configured"
                echo "  → Set --build-repository or providers.devpod.buildRepository"
            fi
            ;;
        aws|gcp|azure)
            if [[ -n "$BUILD_REPOSITORY" ]]; then
                echo "  Build repository: $BUILD_REPOSITORY"
                echo "  → Build and push to registry"
            else
                echo "  ⚠ No build repository configured"
                echo "  → Set --build-repository or providers.devpod.buildRepository"
            fi
            ;;
        *)
            echo "  Build from Dockerfile"
            ;;
    esac
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
    local step=1
    if [[ "$DEVPOD_PROVIDER" != "docker" ]]; then
        local local_cluster
        local_cluster=$(detect_local_cluster)
        if [[ -n "$local_cluster" ]]; then
            echo "  $step. Build Docker image locally"
            ((step++))
            echo "  $step. Load image into $local_cluster"
            ((step++))
        elif [[ -n "$BUILD_REPOSITORY" ]]; then
            echo "  $step. Authenticate with Docker registry"
            ((step++))
            echo "  $step. Build Docker image"
            ((step++))
            echo "  $step. Push to $BUILD_REPOSITORY"
            ((step++))
        fi
    fi
    echo "  $step. Generate .devcontainer/devcontainer.json"
    ((step++))
    echo "  $step. Add '$DEVPOD_PROVIDER' provider to DevPod (if needed)"
    ((step++))
    if [[ "$DEVPOD_PROVIDER" == "kubernetes" ]]; then
        echo "  $step. Create namespace '$K8S_NAMESPACE' (if needed)"
        ((step++))
    fi
    echo "  $step. Run: devpod up . --provider $DEVPOD_PROVIDER --id $NAME"
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

adapter_dispatch "$COMMAND" cmd_deploy cmd_connect cmd_destroy cmd_plan cmd_status show_help
