#!/bin/bash
# Docker adapter - Local Docker deployment
#
# Usage:
#   docker-adapter.sh [OPTIONS] [sindri.yaml]
#
# Options:
#   --config-only    Generate docker-compose.yml without building/deploying
#   --output-dir     Directory for generated files (default: current directory)
#   --output-vars    Output parsed variables for CI integration (JSON to stdout)
#   --skip-build     Skip Docker image build (use existing image)
#   --container-name Override container name from sindri.yaml
#   --help           Show this help message
#
# Examples:
#   docker-adapter.sh                           # Build and deploy using ./sindri.yaml
#   docker-adapter.sh --config-only             # Just generate docker-compose.yml
#   docker-adapter.sh --skip-build              # Deploy without rebuilding image

set -e

# shellcheck disable=SC2034  # May be used in future adapter implementations
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BASE_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Default values
SINDRI_YAML=""
CONFIG_ONLY=false
OUTPUT_DIR="."
OUTPUT_VARS=false
SKIP_BUILD=false
CONTAINER_NAME_OVERRIDE=""

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
        --skip-build)
            SKIP_BUILD=true
            shift
            ;;
        --container-name)
            CONTAINER_NAME_OVERRIDE="$2"
            shift 2
            ;;
        --help)
            head -20 "$0" | tail -17
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

# Parse sindri.yaml
NAME=$(yq '.name' "$SINDRI_YAML")
# Apply container name override if provided
[[ -n "$CONTAINER_NAME_OVERRIDE" ]] && NAME="$CONTAINER_NAME_OVERRIDE"

MEMORY=$(yq '.deployment.resources.memory // "1GB"' "$SINDRI_YAML")
CPUS=$(yq '.deployment.resources.cpus // 1' "$SINDRI_YAML")
PROFILE=$(yq '.extensions.profile // ""' "$SINDRI_YAML")

# Output variables for CI integration if requested
if [[ "$OUTPUT_VARS" == "true" ]]; then
    cat << EOJSON
{
  "name": "$NAME",
  "memory": "$MEMORY",
  "cpus": $CPUS,
  "profile": "$PROFILE"
}
EOJSON
    exit 0
fi

# Ensure output directory exists
mkdir -p "$OUTPUT_DIR"

# Build image unless skipped or config-only
if [[ "$CONFIG_ONLY" != "true" ]] && [[ "$SKIP_BUILD" != "true" ]]; then
    echo "==> Building Docker image with latest source..."
    docker build -t sindri:latest -f docker/Dockerfile .
fi

# Resolve secrets (skip in config-only mode)
if [[ "$CONFIG_ONLY" != "true" ]]; then
    print_status "Resolving secrets..."
    if secrets_resolve_all "$SINDRI_YAML"; then
        print_status "Generating .env.secrets for Docker..."
        secrets_generate_env "$OUTPUT_DIR/.env.secrets"
    fi
fi

# Generate docker-compose.yml with secrets
cat > "$OUTPUT_DIR/docker-compose.yml" << EODC
services:
  sindri:
    image: sindri:latest
    container_name: ${NAME}
    volumes:
      - dev_home:/alt/home/developer
EODC

# Add env_file if secrets exist (check in output dir)
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

# Add file secrets as Docker secrets/mounts (skip in config-only mode)
if [[ "$CONFIG_ONLY" != "true" ]]; then
    secrets_get_docker_mounts >> "$OUTPUT_DIR/docker-compose.yml" || true
fi

# Add resource limits
cat >> "$OUTPUT_DIR/docker-compose.yml" << EODC
    deploy:
      resources:
        limits:
          memory: ${MEMORY}
          cpus: '${CPUS}'
    stdin_open: true
    tty: true
    command: sleep infinity

volumes:
  dev_home:
    driver: local

EODC

# Add file secrets definition (skip in config-only mode)
if [[ "$CONFIG_ONLY" != "true" ]]; then
    secrets_get_docker_files >> "$OUTPUT_DIR/docker-compose.yml" || true
fi

# If config-only mode, just report success and exit
if [[ "$CONFIG_ONLY" == "true" ]]; then
    echo "==> Generated docker-compose.yml at $OUTPUT_DIR/docker-compose.yml"
    echo "    Container name: $NAME"
    echo "    Profile: $PROFILE"
    exit 0
fi

echo "==> Starting Docker container..."
# Use generated docker-compose.yml from output directory if different from current
if [[ "$OUTPUT_DIR" != "." ]]; then
    docker compose -f "$OUTPUT_DIR/docker-compose.yml" up -d
else
    docker compose up -d
fi

echo "==> Container started. Connect with:"
echo "    docker exec -it ${NAME} /bin/bash"
