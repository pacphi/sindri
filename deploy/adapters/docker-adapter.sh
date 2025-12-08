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

# Source common adapter functions
# shellcheck source=adapter-common.sh
source "$(dirname "${BASH_SOURCE[0]}")/adapter-common.sh"

# Initialize adapter
adapter_init "${BASH_SOURCE[0]}"

# Docker-specific defaults
SKIP_BUILD=false
FORCE_REBUILD=false
# shellcheck disable=SC2034  # Used via indirect expansion in adapter_parse_base_config
CONTAINER_NAME_OVERRIDE=""

# Show help wrapper
show_help() {
    adapter_show_help "$0" 30
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
        --config-only)    CONFIG_ONLY=true; shift ;;
        --output-dir)     OUTPUT_DIR="$2"; shift 2 ;;
        --output-vars)    OUTPUT_VARS=true; shift ;;
        --skip-build)     SKIP_BUILD=true; shift ;;
        --rebuild)        FORCE_REBUILD=true; shift ;;
        --container-name) CONTAINER_NAME_OVERRIDE="$2"; shift 2 ;;
        --ci-mode)        CI_MODE=true; shift ;;
        --force|-f)       FORCE=true; shift ;;
        --help|-h)        show_help ;;
        -*)               adapter_unknown_option "$1" ;;
        *)                SINDRI_YAML="$1"; shift ;;
    esac
done

# Validate config file
adapter_validate_config

# Source common utilities (print_*, etc.)
source "$BASE_DIR/docker/lib/common.sh"

# ============================================================================
# Configuration Parsing
# ============================================================================

parse_config() {
    # Parse base configuration
    adapter_parse_base_config "CONTAINER_NAME_OVERRIDE"

    # Docker-specific: Memory is used as-is (e.g., "4GB")
    # MEMORY, CPUS, VOLUME_SIZE already set by adapter_parse_base_config

    # Re-read volume size with GB suffix for docker-compose
    VOLUME_SIZE=$(yq '.deployment.volumes.workspace.size // "10GB"' "$SINDRI_YAML")
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
      - ${NAME}_home:/alt/home/developer
EODC

    # Add env_file if secrets exist
    if [[ -f "$OUTPUT_DIR/.env.secrets" ]] && [[ -s "$OUTPUT_DIR/.env.secrets" ]]; then
        cat >> "$OUTPUT_DIR/docker-compose.yml" << EODC
    env_file:
      - .env.secrets
EODC
    fi

    # Get skip_auto_install value
    local skip_auto_install
    skip_auto_install=$(adapter_get_skip_auto_install)

    # Add environment variables
    cat >> "$OUTPUT_DIR/docker-compose.yml" << EODC
    environment:
      - INIT_WORKSPACE=true
      - INSTALL_PROFILE=${PROFILE}
      - SKIP_AUTO_INSTALL=${skip_auto_install}
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
  ${NAME}_home:
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
    adapter_validate_docker_gpu

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
        if [[ "$FORCE_REBUILD" == "true" ]]; then
            print_status "Forcing rebuild (--no-cache)..."
            docker build --no-cache -t sindri:latest -f Dockerfile .
        else
            docker build -t sindri:latest -f Dockerfile .
        fi
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
    echo "  docker exec -it $NAME /docker/scripts/entrypoint.sh /bin/bash"
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

    # Run through entrypoint to properly setup environment, show MOTD/welcome,
    # switch to developer user, and cd to workspace
    docker exec -it "$NAME" /docker/scripts/entrypoint.sh /bin/bash
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

    # IMPORTANT: Run docker compose down FIRST (before manual container removal)
    # This ensures volumes are properly cleaned up. Manual removal after compose down
    # breaks the association and volumes may be left behind (especially with OrbStack).
    if [[ -f "$OUTPUT_DIR/docker-compose.yml" ]]; then
        print_status "Cleaning up docker-compose resources (container, volumes, networks)..."
        # Use --volumes to remove named volumes and --remove-orphans for stale containers
        docker compose -f "$OUTPUT_DIR/docker-compose.yml" down --volumes --remove-orphans 2>/dev/null || true
    fi

    # Manual container cleanup as fallback (in case compose down didn't fully work)
    if docker ps -a --format '{{.Names}}' | grep -q "^${NAME}$"; then
        print_status "Stopping container..."
        docker stop "$NAME" 2>/dev/null || true
        print_status "Removing container..."
        docker rm "$NAME" 2>/dev/null || true
    fi

    # Clean up generated files
    if [[ -f "$OUTPUT_DIR/docker-compose.yml" ]]; then
        rm -f "$OUTPUT_DIR/docker-compose.yml"
        rm -f "$OUTPUT_DIR/.env.secrets"
    fi

    # Determine the Docker Compose project name
    # Docker Compose uses the directory name where compose file lives as project name
    local project_name
    if [[ "$OUTPUT_DIR" == "." ]]; then
        project_name=$(basename "$(pwd)")
    else
        project_name=$(basename "$OUTPUT_DIR")
    fi

    # Clean up any remaining volumes for this deployment
    # Docker Compose creates volumes as: <project>_<volume_name>
    # Our compose file defines volume as: ${NAME}_home
    # So the actual volume name is: <project>_${NAME}_home
    print_status "Checking for remaining volumes..."

    # Find all volumes that match our deployment patterns
    local volumes_found
    volumes_found=$(docker volume ls --format '{{.Name}}' 2>/dev/null | grep -E "(^${project_name}_${NAME}_home$|^${NAME}_home$|_${NAME}_home$)" || true)

    if [[ -n "$volumes_found" ]]; then
        while IFS= read -r vol; do
            if [[ -n "$vol" ]]; then
                print_status "Removing volume: $vol"
                if ! docker volume rm "$vol" 2>&1; then
                    # Force removal - volume might be "in use" by stopped container reference
                    print_warning "Standard removal failed, attempting force removal..."
                    docker volume rm -f "$vol" 2>/dev/null || print_warning "Could not remove volume $vol"
                fi
            fi
        done <<< "$volumes_found"
    fi

    # Clean up orphaned sindri networks
    for network in $(docker network ls --filter "name=sindri" -q 2>/dev/null); do
        local network_name
        network_name=$(docker network inspect "$network" --format '{{.Name}}' 2>/dev/null)
        if [[ -n "$network_name" ]]; then
            print_status "Removing network: $network_name"
            docker network rm "$network" 2>/dev/null || true
        fi
    done

    # Also clean up networks created by compose (project_default pattern)
    for network in $(docker network ls --filter "name=${project_name}" -q 2>/dev/null); do
        local network_name
        network_name=$(docker network inspect "$network" --format '{{.Name}}' 2>/dev/null)
        if [[ -n "$network_name" ]]; then
            print_status "Removing network: $network_name"
            docker network rm "$network" 2>/dev/null || true
        fi
    done

    print_success "Container and volumes destroyed"
}

cmd_plan() {
    parse_config

    print_header "Docker Deployment Plan"
    echo ""
    echo "Container:  $NAME"
    echo "Profile:    $PROFILE"
    echo "Auto-install: $AUTO_INSTALL"
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
    if [[ "$AUTO_INSTALL" == "true" ]]; then
        echo "  6. Auto-install extension profile: $PROFILE"
    else
        echo "  6. Skip auto-install (manual: extension-manager install-profile $PROFILE)"
    fi
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

adapter_dispatch "$COMMAND" cmd_deploy cmd_connect cmd_destroy cmd_plan cmd_status show_help
