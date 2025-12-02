#!/bin/bash
# Docker setup helper for GitHub Actions
# Generates docker-compose.yml using adapter or creates minimal fallback config
#
# Usage:
#   docker-setup.sh --container-name NAME [--sindri-config PATH] [--output-dir DIR]
#
# When --sindri-config is provided, uses the docker-adapter.sh to generate comprehensive config.
# Otherwise, creates a minimal docker-compose.yml suitable for CI testing.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# Default values
CONTAINER_NAME=""
SINDRI_CONFIG=""
OUTPUT_DIR="."

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --container-name)
            CONTAINER_NAME="$2"
            shift 2
            ;;
        --sindri-config)
            SINDRI_CONFIG="$2"
            shift 2
            ;;
        --output-dir)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --help)
            head -12 "$0" | tail -10
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

# Validate required inputs
if [[ -z "$CONTAINER_NAME" ]]; then
    echo "Error: --container-name is required" >&2
    exit 1
fi

# Ensure output directory exists
mkdir -p "$OUTPUT_DIR"

# Check if docker-compose.yml already exists
if [[ -f "$OUTPUT_DIR/docker-compose.yml" ]]; then
    echo "source=existing"
    echo "Using existing docker-compose.yml"
    exit 0
fi

# If sindri-config provided and exists, use the adapter
if [[ -n "$SINDRI_CONFIG" ]] && [[ -f "$SINDRI_CONFIG" ]]; then
    echo "source=adapter"
    echo "Generating docker-compose.yml using adapter with config: $SINDRI_CONFIG"

    "$REPO_ROOT/deploy/adapters/docker-adapter.sh" \
        --config-only \
        --output-dir "$OUTPUT_DIR" \
        --container-name "$CONTAINER_NAME" \
        "$SINDRI_CONFIG"

    exit 0
fi

# Fallback: Use the docker-adapter with a minimal inline config
echo "source=fallback"
echo "Generating minimal docker-compose.yml (no sindri-config provided)"

TEMP_CONFIG=$(mktemp)
cat > "$TEMP_CONFIG" << YAML
version: "1.0"
name: ${CONTAINER_NAME}
deployment:
  provider: docker
  resources:
    memory: 4GB
    cpus: 2
extensions:
  profile: minimal
YAML

# Use the adapter with the temporary config
"$REPO_ROOT/deploy/adapters/docker-adapter.sh" \
    --config-only \
    --output-dir "$OUTPUT_DIR" \
    --container-name "$CONTAINER_NAME" \
    "$TEMP_CONFIG"

# Cleanup
rm -f "$TEMP_CONFIG"

echo "Generated docker-compose.yml successfully"
