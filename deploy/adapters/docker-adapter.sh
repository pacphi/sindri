#!/bin/bash
# Docker adapter - Local Docker deployment

set -e

# shellcheck disable=SC2034  # May be used in future adapter implementations
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SINDRI_YAML="${1:-sindri.yaml}"

if [[ ! -f "$SINDRI_YAML" ]]; then
    echo "Error: $SINDRI_YAML not found"
    exit 1
fi

# Parse sindri.yaml
NAME=$(yq '.name' "$SINDRI_YAML")
MEMORY=$(yq '.deployment.resources.memory // "1GB"' "$SINDRI_YAML")
CPUS=$(yq '.deployment.resources.cpus // 1' "$SINDRI_YAML")
PROFILE=$(yq '.extensions.profile // ""' "$SINDRI_YAML")

# Always rebuild for local development to ensure fresh source
echo "==> Building Docker image with latest source..."
docker build -t sindri:latest -f docker/Dockerfile .

# Generate docker-compose.yml
cat > docker-compose.yml << EODC
services:
  sindri:
    image: sindri:latest
    container_name: ${NAME}
    volumes:
      - workspace:/workspace
    environment:
      - INIT_WORKSPACE=true
      - INSTALL_PROFILE=${PROFILE}
    deploy:
      resources:
        limits:
          memory: ${MEMORY}
          cpus: '${CPUS}'
    stdin_open: true
    tty: true
    command: sleep infinity

volumes:
  workspace:
    driver: local
EODC

echo "==> Starting Docker container..."
docker-compose up -d

echo "==> Container started. Connect with:"
echo "    docker exec -it ${NAME} /bin/bash"
