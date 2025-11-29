#!/bin/bash
# Docker adapter - Local Docker deployment

set -e

# shellcheck disable=SC2034  # May be used in future adapter implementations
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BASE_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
SINDRI_YAML="${1:-sindri.yaml}"

if [[ ! -f "$SINDRI_YAML" ]]; then
    echo "Error: $SINDRI_YAML not found"
    exit 1
fi

# Source common utilities and secrets manager
source "$BASE_DIR/docker/lib/common.sh"
source "$BASE_DIR/cli/secrets-manager"

# Parse sindri.yaml
NAME=$(yq '.name' "$SINDRI_YAML")
MEMORY=$(yq '.deployment.resources.memory // "1GB"' "$SINDRI_YAML")
CPUS=$(yq '.deployment.resources.cpus // 1' "$SINDRI_YAML")
PROFILE=$(yq '.extensions.profile // ""' "$SINDRI_YAML")

# Always rebuild for local development to ensure fresh source
echo "==> Building Docker image with latest source..."
docker build -t sindri:latest -f docker/Dockerfile .

# Resolve secrets
print_status "Resolving secrets..."
if secrets_resolve_all "$SINDRI_YAML"; then
    print_status "Generating .env.secrets for Docker..."
    secrets_generate_env ".env.secrets"
fi

# Generate docker-compose.yml with secrets
cat > docker-compose.yml << EODC
services:
  sindri:
    image: sindri:latest
    container_name: ${NAME}
    volumes:
      - dev_home:/alt/home/developer
EODC

# Add env_file if secrets exist
if [[ -f .env.secrets ]] && [[ -s .env.secrets ]]; then
    cat >> docker-compose.yml << EODC
    env_file:
      - .env.secrets
EODC
fi

# Add environment variables
cat >> docker-compose.yml << EODC
    environment:
      - INIT_WORKSPACE=true
      - INSTALL_PROFILE=${PROFILE}
EODC

# Add file secrets as Docker secrets/mounts
secrets_get_docker_mounts >> docker-compose.yml || true

# Add resource limits
cat >> docker-compose.yml << EODC
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

# Add file secrets definition
secrets_get_docker_files >> docker-compose.yml || true

echo "==> Starting Docker container..."
docker compose up -d

echo "==> Container started. Connect with:"
echo "    docker exec -it ${NAME} /bin/bash"
