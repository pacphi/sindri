#!/bin/bash
# Docker adapter - Full lifecycle management for local Docker deployments
#
# Usage:
#   docker-adapter.sh <command> [OPTIONS] [sindri.yaml]
#
# Commands:
#   deploy     Build and start container
#   connect    Exec into container
#   destroy    Stop and remove container
#   plan       Show deployment plan
#   status     Show container status
#
# Options:
#   --config-only    Generate docker-compose.yml without deploying (deploy only)
#   --output-dir     Directory for generated files (default: current directory)
#   --output-vars    Output parsed variables as JSON (deploy only)
#   --skip-build     Skip Docker image build (deploy only)
#   --container-name Override container name from sindri.yaml
#   --force          Skip confirmation prompts (destroy only)
#   --help           Show this help message
#
# Examples:
#   docker-adapter.sh deploy                    # Build and deploy
#   docker-adapter.sh deploy --config-only     # Just generate docker-compose.yml
#   docker-adapter.sh deploy --skip-build      # Deploy without rebuilding
#   docker-adapter.sh status                    # Show container status
#   docker-adapter.sh destroy --force           # Teardown without confirmation

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BASE_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

COMMAND=""
SINDRI_YAML=""
CONFIG_ONLY=false
OUTPUT_DIR="."
OUTPUT_VARS=false
SKIP_BUILD=false
CONTAINER_NAME_OVERRIDE=""
FORCE=false

show_help() {
    head -30 "$0" | tail -28
    exit 0
}

[[ $# -eq 0 ]] && show_help

COMMAND="$1"
shift

while [[ $# -gt 0 ]]; do
    case $1 in
        --config-only)    CONFIG_ONLY=true; shift ;;
        --output-dir)     OUTPUT_DIR="$2"; shift 2 ;;
        --output-vars)    OUTPUT_VARS=true; shift ;;
        --skip-build)     SKIP_BUILD=true; shift ;;
        --container-name) CONTAINER_NAME_OVERRIDE="$2"; shift 2 ;;
        --force|-f)       FORCE=true; shift ;;
        --help|-h)        show_help ;;
        -*)               echo "Unknown option: $1" >&2; exit 1 ;;
        *)                SINDRI_YAML="$1"; shift ;;
    esac
done

SINDRI_YAML="${SINDRI_YAML:-sindri.yaml}"

if [[ ! -f "$SINDRI_YAML" ]]; then
    echo "Error: $SINDRI_YAML not found" >&2
    exit 1
fi

source "$BASE_DIR/docker/lib/common.sh"

# ============================================================================
# Configuration Parsing
# ============================================================================

parse_config() {
    NAME=$(yq '.name' "$SINDRI_YAML")
    if [[ -n "$CONTAINER_NAME_OVERRIDE" ]]; then
        NAME="$CONTAINER_NAME_OVERRIDE"
    fi

    MEMORY=$(yq '.deployment.resources.memory // "1GB"' "$SINDRI_YAML")
    CPUS=$(yq '.deployment.resources.cpus // 1' "$SINDRI_YAML")
    PROFILE=$(yq '.extensions.profile // "minimal"' "$SINDRI_YAML")
    VOLUME_SIZE=$(yq '.deployment.volumes.workspace.size // "10GB"' "$SINDRI_YAML")

    # GPU configuration
    GPU_ENABLED=$(yq '.deployment.resources.gpu.enabled // false' "$SINDRI_YAML")
    GPU_TYPE=$(yq '.deployment.resources.gpu.type // "nvidia"' "$SINDRI_YAML")
    GPU_COUNT=$(yq '.deployment.resources.gpu.count // 1' "$SINDRI_YAML")
}

validate_gpu() {
    if [[ "$GPU_ENABLED" != "true" ]]; then
        return 0
    fi

    if [[ "$GPU_TYPE" != "nvidia" ]]; then
        print_error "Docker adapter only supports nvidia GPUs (requested: $GPU_TYPE)"
        exit 1
    fi

    # Check for NVIDIA runtime availability
    if ! docker info 2>/dev/null | grep -q "nvidia"; then
        print_warning "NVIDIA Docker runtime not detected"
        echo "Install nvidia-container-toolkit for GPU support." >&2
    fi
}

# ============================================================================
# docker-compose.yml Generation
# ============================================================================

generate_compose() {
    mkdir -p "$OUTPUT_DIR"

    # Start docker-compose.yml
    cat > "$OUTPUT_DIR/docker-compose.yml" << EODC
# Docker Compose configuration for Sindri
# Local development environment with persistent storage

services:
  sindri:
    image: sindri:latest
    container_name: ${NAME}
    volumes:
      - dev_home:/alt/home/developer
EODC

    # Add env_file if secrets exist
    if [[ -f "$OUTPUT_DIR/.env.secrets" ]] && [[ -s "$OUTPUT_DIR/.env.secrets" ]]; then
        cat >> "$OUTPUT_DIR/docker-compose.yml" << EODC
    env_file:
      - .env.secrets
EODC
    fi

    # Add environment variables
    cat >> "$OUTPUT_DIR/docker-compose.yml" << EODC
    environment:
      - INIT_WORKSPACE=true
      - INSTALL_PROFILE=${PROFILE}
EODC

    # Add GPU or standard resource configuration
    if [[ "$GPU_ENABLED" == "true" ]]; then
        cat >> "$OUTPUT_DIR/docker-compose.yml" << EODC
    # GPU-enabled configuration
    runtime: nvidia
    deploy:
      resources:
        limits:
          memory: ${MEMORY}
          cpus: '${CPUS}'
        reservations:
          devices:
            - driver: nvidia
              count: ${GPU_COUNT}
              capabilities: [gpu, compute, utility]
EODC
    else
        cat >> "$OUTPUT_DIR/docker-compose.yml" << EODC
    deploy:
      resources:
        limits:
          memory: ${MEMORY}
          cpus: '${CPUS}'
EODC
    fi

    # Add container settings and volume definition
    cat >> "$OUTPUT_DIR/docker-compose.yml" << EODC
    stdin_open: true
    tty: true
    command: sleep infinity

volumes:
  dev_home:
    driver: local
EODC
}

# ============================================================================
# Commands
# ============================================================================

cmd_deploy() {
    parse_config

    # Output variables for CI integration
    if [[ "$OUTPUT_VARS" == "true" ]]; then
        cat << EOJSON
{
  "name": "$NAME",
  "memory": "$MEMORY",
  "cpus": $CPUS,
  "profile": "$PROFILE",
  "gpu_enabled": $GPU_ENABLED,
  "gpu_type": "$GPU_TYPE",
  "gpu_count": $GPU_COUNT
}
EOJSON
        exit 0
    fi

    # Validate GPU if enabled
    validate_gpu

    # Resolve secrets (skip in config-only mode)
    if [[ "$CONFIG_ONLY" != "true" ]]; then
        source "$BASE_DIR/cli/secrets-manager"
        print_status "Resolving secrets..."
        if secrets_resolve_all "$SINDRI_YAML"; then
            print_status "Generating .env.secrets for Docker..."
            secrets_generate_env "$OUTPUT_DIR/.env.secrets"
        fi
    fi

    generate_compose

    if [[ "$CONFIG_ONLY" == "true" ]]; then
        print_success "Generated docker-compose.yml at $OUTPUT_DIR/"
        echo "  Container: $NAME"
        echo "  Profile: $PROFILE"
        if [[ "$GPU_ENABLED" == "true" ]]; then
            echo "  GPU: $GPU_TYPE (count: $GPU_COUNT)"
        fi
        return 0
    fi

    print_header "Deploying with Docker"
    echo "  Container: $NAME"
    echo "  Profile: $PROFILE"
    echo "  Resources: ${CPUS} CPUs, ${MEMORY} memory"
    if [[ "$GPU_ENABLED" == "true" ]]; then
        echo "  GPU: $GPU_TYPE (count: $GPU_COUNT)"
    fi
    echo ""

    # Build image unless skipped
    if [[ "$SKIP_BUILD" != "true" ]]; then
        print_status "Building Docker image..."
        docker build -t sindri:latest -f Dockerfile .
    fi

    # Start container
    print_status "Starting container..."
    if [[ "$OUTPUT_DIR" != "." ]]; then
        docker compose -f "$OUTPUT_DIR/docker-compose.yml" up -d
    else
        docker compose up -d
    fi

    print_success "Container '$NAME' deployed successfully"
    echo ""
    echo "Connect:"
    echo "  sindri connect"
    echo "  docker exec -it $NAME /bin/bash"
    echo ""
    echo "Manage:"
    echo "  sindri status"
    echo "  sindri destroy"
}

cmd_connect() {
    parse_config

    if ! docker ps --format '{{.Names}}' | grep -q "^${NAME}$"; then
        print_error "Container '$NAME' not running"
        echo "Deploy first: sindri deploy --provider docker"
        exit 1
    fi

    docker exec -it "$NAME" /bin/bash
}

cmd_destroy() {
    parse_config

    if [[ "$FORCE" != "true" ]]; then
        print_warning "This will destroy container '$NAME' and remove volumes"
        read -p "Are you sure? (y/N) " -n 1 -r
        echo
        [[ ! $REPLY =~ ^[Yy]$ ]] && { print_status "Cancelled"; exit 0; }
    fi

    print_header "Destroying Docker container: $NAME"

    if docker ps -a --format '{{.Names}}' | grep -q "^${NAME}$"; then
        print_status "Stopping container..."
        docker stop "$NAME" 2>/dev/null || true
        print_status "Removing container..."
        docker rm "$NAME" 2>/dev/null || true
    else
        print_warning "Container '$NAME' not found"
    fi

    # Clean up docker-compose resources
    if [[ -f "$OUTPUT_DIR/docker-compose.yml" ]]; then
        print_status "Cleaning up docker-compose resources..."
        docker compose -f "$OUTPUT_DIR/docker-compose.yml" down -v 2>/dev/null || true
        rm -f "$OUTPUT_DIR/docker-compose.yml"
        rm -f "$OUTPUT_DIR/.env.secrets"
    fi

    print_success "Container destroyed"
}

cmd_plan() {
    parse_config

    print_header "Docker Deployment Plan"
    echo ""
    echo "Container:  $NAME"
    echo "Profile:    $PROFILE"
    echo ""
    echo "Resources:"
    echo "  CPUs:     $CPUS"
    echo "  Memory:   $MEMORY"
    echo "  Volume:   $VOLUME_SIZE"
    if [[ "$GPU_ENABLED" == "true" ]]; then
        echo "  GPU:      $GPU_TYPE (count: $GPU_COUNT)"
    fi
    echo ""
    echo "Actions:"
    echo "  1. Build sindri:latest image"
    echo "  2. Generate docker-compose.yml"
    echo "  3. Resolve and inject secrets"
    echo "  4. Create volume: dev_home"
    echo "  5. Start container: $NAME"
    echo "  6. Install extension profile: $PROFILE"
}

cmd_status() {
    parse_config

    print_header "Docker Deployment Status"
    echo ""
    echo "Container: $NAME"
    echo ""

    if docker ps -a --format '{{.Names}}' | grep -q "^${NAME}$"; then
        docker ps -a --filter "name=^${NAME}$" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
        echo ""

        local state
        state=$(docker inspect -f '{{.State.Status}}' "$NAME" 2>/dev/null || echo "unknown")
        echo "State: $state"

        if [[ "$state" == "running" ]]; then
            echo ""
            echo "Resource usage:"
            docker stats --no-stream --format "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}" "$NAME" 2>/dev/null || true
        fi
    else
        echo "Status: Not deployed"
        echo ""
        echo "Deploy with: sindri deploy --provider docker"
    fi
}

# ============================================================================
# Main Dispatch
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
