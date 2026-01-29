#!/usr/bin/env bash
# K3d Cluster Manager for Sindri v3 CI
# Manages k3d cluster lifecycle with local registry for fast pod pulls
set -euo pipefail

# Defaults
DEFAULT_CLUSTER_NAME="sindri-test"
DEFAULT_REGISTRY_NAME="sindri-registry"
DEFAULT_REGISTRY_PORT="5050"
DEFAULT_AGENTS=0
DEFAULT_K3D_VERSION="v5.7.4"
DEFAULT_K3S_VERSION="v1.30.4-k3s1"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $*" >&2; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*" >&2; }
log_error() { echo -e "${RED}[ERROR]${NC} $*" >&2; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $*" >&2; }

usage() {
  cat <<EOF
K3d Cluster Manager for Sindri v3 CI

USAGE:
    $(basename "$0") <command> [options]

COMMANDS:
    install                 Install k3d if not present
    create <name> [agents]  Create a new k3d cluster with local registry
    push <name> <image>     Push image to local registry
    kubeconfig <name>       Output kubeconfig for cluster
    destroy <name>          Delete cluster and registry
    status <name>           Check cluster status
    list                    List all clusters

OPTIONS:
    --registry-port <port>  Registry port (default: $DEFAULT_REGISTRY_PORT)
    --k3s-version <version> K3s version (default: $DEFAULT_K3S_VERSION)

EXAMPLES:
    $(basename "$0") install
    $(basename "$0") create test-cluster 1
    $(basename "$0") push test-cluster sindri:latest
    $(basename "$0") destroy test-cluster
EOF
  exit 1
}

# Install k3d
cmd_install() {
  if command -v k3d &>/dev/null; then
    local version
    version=$(k3d version | head -1 | awk '{print $3}')
    log_info "k3d already installed: $version"
    return 0
  fi

  log_info "Installing k3d ${DEFAULT_K3D_VERSION}..."

  if [[ "$(uname)" == "Darwin" ]]; then
    if command -v brew &>/dev/null; then
      brew install k3d
    else
      curl -s https://raw.githubusercontent.com/k3d-io/k3d/main/install.sh | TAG="${DEFAULT_K3D_VERSION}" bash
    fi
  else
    curl -s https://raw.githubusercontent.com/k3d-io/k3d/main/install.sh | TAG="${DEFAULT_K3D_VERSION}" bash
  fi

  log_success "k3d installed successfully"
  k3d version
}

# Create cluster with registry
cmd_create() {
  local cluster_name="${1:-$DEFAULT_CLUSTER_NAME}"
  local agents="${2:-$DEFAULT_AGENTS}"
  local registry_name="${DEFAULT_REGISTRY_NAME}"
  local registry_port="${REGISTRY_PORT:-$DEFAULT_REGISTRY_PORT}"
  local k3s_version="${K3S_VERSION:-$DEFAULT_K3S_VERSION}"

  log_info "Creating k3d cluster: $cluster_name"
  log_info "  Agents: $agents"
  log_info "  Registry: $registry_name:$registry_port"
  log_info "  K3s version: $k3s_version"

  # Check if cluster already exists
  if k3d cluster list | grep -q "^${cluster_name}\s"; then
    log_warn "Cluster $cluster_name already exists"
    return 0
  fi

  # Create registry if it doesn't exist
  if ! k3d registry list 2>/dev/null | grep -q "^${registry_name}\s"; then
    log_info "Creating registry: $registry_name"
    k3d registry create "$registry_name" --port "$registry_port"
  else
    log_info "Registry $registry_name already exists"
  fi

  # Create cluster with registry
  k3d cluster create "$cluster_name" \
    --registry-use "k3d-${registry_name}:${registry_port}" \
    --agents "$agents" \
    --image "rancher/k3s:${k3s_version}" \
    --wait \
    --timeout 300s \
    --api-port 6443 \
    --no-lb \
    --k3s-arg "--disable=traefik@server:0" \
    --k3s-arg "--disable=servicelb@server:0"

  # Wait for cluster to be ready
  log_info "Waiting for cluster to be ready..."
  local retries=30
  while [[ $retries -gt 0 ]]; do
    if kubectl --context "k3d-${cluster_name}" get nodes 2>/dev/null | grep -q "Ready"; then
      break
    fi
    sleep 2
    ((retries--))
  done

  if [[ $retries -eq 0 ]]; then
    log_error "Cluster failed to become ready"
    return 1
  fi

  log_success "Cluster $cluster_name created successfully"
  kubectl --context "k3d-${cluster_name}" get nodes
}

# Push image to local registry
cmd_push() {
  local cluster_name="${1:-$DEFAULT_CLUSTER_NAME}"
  local image="${2:-}"
  local registry_port="${REGISTRY_PORT:-$DEFAULT_REGISTRY_PORT}"

  if [[ -z "$image" ]]; then
    log_error "Image name required"
    exit 1
  fi

  local registry_image="localhost:${registry_port}/${image}"

  log_info "Tagging image: $image -> $registry_image"
  docker tag "$image" "$registry_image" 2>/dev/null || {
    log_warn "Image $image not found locally, attempting to pull..."
    docker pull "$image"
    docker tag "$image" "$registry_image"
  }

  log_info "Pushing image to registry: $registry_image"
  docker push "$registry_image"

  log_success "Image pushed: $registry_image"
  echo "$registry_image"
}

# Get kubeconfig
cmd_kubeconfig() {
  local cluster_name="${1:-$DEFAULT_CLUSTER_NAME}"

  if ! k3d cluster list | grep -q "^${cluster_name}\s"; then
    log_error "Cluster $cluster_name not found"
    exit 1
  fi

  k3d kubeconfig get "$cluster_name"
}

# Destroy cluster
cmd_destroy() {
  local cluster_name="${1:-$DEFAULT_CLUSTER_NAME}"
  local registry_name="${DEFAULT_REGISTRY_NAME}"

  log_info "Destroying cluster: $cluster_name"

  if k3d cluster list | grep -q "^${cluster_name}\s"; then
    k3d cluster delete "$cluster_name"
    log_success "Cluster $cluster_name deleted"
  else
    log_warn "Cluster $cluster_name not found"
  fi

  # Optionally delete registry
  if [[ "${DELETE_REGISTRY:-false}" == "true" ]]; then
    if k3d registry list 2>/dev/null | grep -q "^${registry_name}\s"; then
      log_info "Deleting registry: $registry_name"
      k3d registry delete "$registry_name"
      log_success "Registry $registry_name deleted"
    fi
  fi
}

# Check cluster status
cmd_status() {
  local cluster_name="${1:-$DEFAULT_CLUSTER_NAME}"

  if ! k3d cluster list | grep -q "^${cluster_name}\s"; then
    log_error "Cluster $cluster_name not found"
    exit 1
  fi

  log_info "Cluster status: $cluster_name"
  k3d cluster list | grep "^${cluster_name}\s"

  log_info "Node status:"
  kubectl --context "k3d-${cluster_name}" get nodes -o wide 2>/dev/null || log_warn "Cannot connect to cluster"

  log_info "Pod status (all namespaces):"
  kubectl --context "k3d-${cluster_name}" get pods -A 2>/dev/null || log_warn "Cannot get pods"
}

# List clusters
cmd_list() {
  log_info "K3d clusters:"
  k3d cluster list

  log_info "K3d registries:"
  k3d registry list 2>/dev/null || echo "No registries"
}

# Deploy pod for extension testing
cmd_deploy_pod() {
  local cluster_name="${1:-$DEFAULT_CLUSTER_NAME}"
  local pod_name="${2:-sindri-test}"
  local image="${3:-localhost:${DEFAULT_REGISTRY_PORT}/sindri:latest}"
  local memory="${4:-512Mi}"
  local cpu="${5:-500m}"

  log_info "Deploying pod: $pod_name"
  log_info "  Image: $image"
  log_info "  Memory: $memory"
  log_info "  CPU: $cpu"

  kubectl --context "k3d-${cluster_name}" apply -f - <<EOF
apiVersion: v1
kind: Pod
metadata:
  name: ${pod_name}
  labels:
    app: sindri-test
spec:
  containers:
  - name: sindri
    image: ${image}
    imagePullPolicy: Always
    resources:
      requests:
        memory: "${memory}"
        cpu: "${cpu}"
      limits:
        memory: "${memory}"
        cpu: "${cpu}"
    command: ["sleep", "infinity"]
  restartPolicy: Never
EOF

  # Wait for pod to be ready
  log_info "Waiting for pod to be ready..."
  kubectl --context "k3d-${cluster_name}" wait --for=condition=Ready "pod/${pod_name}" --timeout=120s

  log_success "Pod $pod_name deployed"
}

# Delete pod
cmd_delete_pod() {
  local cluster_name="${1:-$DEFAULT_CLUSTER_NAME}"
  local pod_name="${2:-sindri-test}"

  log_info "Deleting pod: $pod_name"
  kubectl --context "k3d-${cluster_name}" delete pod "$pod_name" --ignore-not-found --wait=false
  log_success "Pod $pod_name deleted"
}

# Execute command in pod
cmd_exec() {
  local cluster_name="${1:-$DEFAULT_CLUSTER_NAME}"
  local pod_name="${2:-sindri-test}"
  shift 2
  local cmd="$*"

  kubectl --context "k3d-${cluster_name}" exec -it "$pod_name" -- bash -c "$cmd"
}

# Main
main() {
  local cmd="${1:-}"
  shift || true

  # Parse global options
  REGISTRY_PORT="$DEFAULT_REGISTRY_PORT"
  K3S_VERSION="$DEFAULT_K3S_VERSION"
  DELETE_REGISTRY=false

  local args=()
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --registry-port)
        REGISTRY_PORT="$2"
        shift 2
        ;;
      --k3s-version)
        K3S_VERSION="$2"
        shift 2
        ;;
      --delete-registry)
        DELETE_REGISTRY=true
        shift
        ;;
      *)
        args+=("$1")
        shift
        ;;
    esac
  done

  export REGISTRY_PORT K3S_VERSION DELETE_REGISTRY

  case "$cmd" in
    install)
      cmd_install
      ;;
    create)
      cmd_create "${args[@]:-}"
      ;;
    push)
      [[ ${#args[@]} -lt 2 ]] && { log_error "Cluster name and image required"; exit 1; }
      cmd_push "${args[0]}" "${args[1]}"
      ;;
    kubeconfig)
      cmd_kubeconfig "${args[0]:-}"
      ;;
    destroy)
      cmd_destroy "${args[0]:-}"
      ;;
    status)
      cmd_status "${args[0]:-}"
      ;;
    list)
      cmd_list
      ;;
    deploy-pod)
      cmd_deploy_pod "${args[@]:-}"
      ;;
    delete-pod)
      cmd_delete_pod "${args[@]:-}"
      ;;
    exec)
      cmd_exec "${args[@]:-}"
      ;;
    -h|--help|help)
      usage
      ;;
    "")
      usage
      ;;
    *)
      log_error "Unknown command: $cmd"
      usage
      ;;
  esac
}

main "$@"
